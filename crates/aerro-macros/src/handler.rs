//! `#[aerro::handler]` — inline adapter that wraps a user handler so its
//! typed error becomes a `tonic::Status` with the route's encode options.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::Parser;
use syn::{FnArg, Ident, ItemFn, parse2};

#[derive(Default)]
struct HandlerArgs {
    service: Option<String>,
    rpc: Option<String>,
    exposure: Option<Ident>,
    max_frames: Option<u8>,
}

fn parse_args(args: TokenStream) -> syn::Result<HandlerArgs> {
    let mut out = HandlerArgs::default();
    if args.is_empty() {
        return Ok(out);
    }
    let parser = syn::meta::parser(|meta| {
        let key = meta
            .path
            .get_ident()
            .ok_or_else(|| meta.error("expected an identifier"))?
            .to_string();
        match key.as_str() {
            "service" => {
                let v: syn::LitStr = meta.value()?.parse()?;
                out.service = Some(v.value());
            }
            "rpc" => {
                let v: syn::LitStr = meta.value()?.parse()?;
                out.rpc = Some(v.value());
            }
            "exposure" => {
                let v: Ident = meta.value()?.parse()?;
                out.exposure = Some(v);
            }
            "max_frames" => {
                let v: syn::LitInt = meta.value()?.parse()?;
                out.max_frames = Some(v.base10_parse()?);
            }
            other => return Err(meta.error(format!("unknown handler arg `{}`", other))),
        }
        Ok(())
    });
    parser.parse2(args)?;
    Ok(out)
}

pub fn expand(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };
    let user_fn: ItemFn = match parse2(item.clone()) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error(),
    };

    let service = args.service.unwrap_or_else(|| "unknown".into());
    let rpc = args.rpc.unwrap_or_else(|| user_fn.sig.ident.to_string());
    let exposure_ident = if let Some(ident) = args.exposure {
        match ident.to_string().as_str() {
            "Internal" | "Trusted" | "Public" => ident,
            other => {
                return syn::Error::new(
                    ident.span(),
                    format!("unknown exposure `{}` (expected one of: Internal, Trusted, Public)", other),
                )
                .to_compile_error();
            }
        }
    } else {
        Ident::new("Internal", proc_macro2::Span::call_site())
    };
    let max_frames = args.max_frames.unwrap_or(16);

    let outer_ident = user_fn.sig.ident.clone();
    let inner_ident = format_ident!("__aerro_inner_{}", outer_ident);

    // Build inner fn: rename outer -> inner, keep async, generics, args, body.
    let mut inner_fn = user_fn.clone();
    inner_fn.sig.ident = inner_ident.clone();
    inner_fn.vis = syn::Visibility::Inherited;
    for a in &mut inner_fn.attrs {
        let _ = a;
    }

    // Snapshot the outer signature so we don't fight the borrow checker when
    // synthesising the wrapper.
    let outer_vis = user_fn.vis.clone();
    let outer_attrs = user_fn.attrs.clone();
    let outer_asyncness = user_fn.sig.asyncness;
    let outer_inputs = user_fn.sig.inputs.clone();
    let outer_generics = user_fn.sig.generics.clone();
    let where_clause = user_fn.sig.generics.where_clause.clone();

    // Extract return type Result<T, E>.
    let (ok_ty, err_ty) = match &user_fn.sig.output {
        syn::ReturnType::Type(_, ty) => match extract_result_args(ty) {
            Some(v) => v,
            None => {
                return syn::Error::new_spanned(
                    ty,
                    "#[aerro::handler] requires a `Result<T, E>` return type",
                )
                .to_compile_error();
            }
        },
        syn::ReturnType::Default => {
            return syn::Error::new_spanned(
                &user_fn.sig.ident,
                "#[aerro::handler] requires a `Result<T, E>` return type",
            )
            .to_compile_error();
        }
    };

    // Forward argument idents to the inner call.
    let forward_args: Vec<TokenStream> = outer_inputs
        .iter()
        .map(|arg| match arg {
            FnArg::Receiver(_) => quote! { self },
            FnArg::Typed(p) => {
                let pat = &p.pat;
                quote! { #pat }
            }
        })
        .collect();

    let wrapper_body = quote! {
        let __opts = ::aerro::wire::encode::EncodeOptions {
            exposure: ::aerro::Exposure::#exposure_ident,
            max_frames: #max_frames,
        };
        let __res: ::core::result::Result<#ok_ty, #err_ty> =
            #inner_ident(#(#forward_args),*).await;
        match __res {
            ::core::result::Result::Ok(__v) => ::core::result::Result::Ok(__v),
            ::core::result::Result::Err(__e) => {
                let mut __sf = ::aerro::ServiceFailure::new(__e);
                let __cat = ::aerro::Aerro::category(&__sf.inner);
                let __code = ::aerro::Aerro::code(&__sf.inner);
                let __msg = ::std::string::ToString::to_string(&__sf.inner);
                __sf.frames.push(::aerro::Frame::local(#service, #rpc, __code, __msg, __cat));
                ::core::result::Result::Err(
                    <::aerro::ServiceFailure<#err_ty> as ::aerro::IntoStatus>::into_status(__sf, &__opts)
                )
            }
        }
    };

    quote! {
        #inner_fn

        // `tonic::Status` is large; the lint flags `Result<_, Status>` but we
        // must return that shape to satisfy tonic's RPC handler signature.
        #[allow(clippy::result_large_err)]
        #(#outer_attrs)*
        #outer_vis #outer_asyncness fn #outer_ident #outer_generics ( #outer_inputs )
            -> ::core::result::Result<#ok_ty, ::tonic::Status>
            #where_clause
        {
            #wrapper_body
        }
    }
}

fn extract_result_args(ty: &syn::Type) -> Option<(syn::Type, syn::Type)> {
    let path = match ty {
        syn::Type::Path(p) => &p.path,
        _ => return None,
    };
    let seg = path.segments.last()?;
    if seg.ident != "Result" {
        return None;
    }
    let args = match &seg.arguments {
        syn::PathArguments::AngleBracketed(a) => &a.args,
        _ => return None,
    };
    let mut iter = args.iter();
    let ok = iter.next()?;
    let err = iter.next()?;
    let ok_ty = match ok {
        syn::GenericArgument::Type(t) => t.clone(),
        _ => return None,
    };
    let err_ty = match err {
        syn::GenericArgument::Type(t) => t.clone(),
        _ => return None,
    };
    Some((ok_ty, err_ty))
}

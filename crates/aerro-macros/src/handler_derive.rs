//! `#[derive(AerroHandler)]` — generates `AerroHandler` metadata impl and
//! an inherent `call_tonic` method on the annotated struct.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Ident};

use crate::attrs::ExposureAttr;

#[derive(Default)]
struct HandlerAttrs {
    service: Option<String>,
    rpc: Option<String>,
    exposure: Option<ExposureAttr>,
    max_frames: Option<u8>,
}

fn parse_aerro_attrs(input: &DeriveInput) -> syn::Result<HandlerAttrs> {
    let mut out = HandlerAttrs::default();

    for attr in &input.attrs {
        if !attr.path().is_ident("aerro") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
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
                    out.exposure = Some(ExposureAttr::from_ident(&v)?);
                }
                "max_frames" => {
                    let v: syn::LitInt = meta.value()?.parse()?;
                    out.max_frames = Some(v.base10_parse()?);
                }
                other => {
                    return Err(meta.error(format!("unknown AerroHandler attribute `{}`", other)));
                }
            }
            Ok(())
        })?;
    }

    Ok(out)
}

pub fn expand(input: TokenStream) -> TokenStream {
    let input: DeriveInput = match syn::parse2(input) {
        Ok(d) => d,
        Err(e) => return e.to_compile_error(),
    };

    // Only structs are supported.
    if !matches!(input.data, Data::Struct(_)) {
        return syn::Error::new(
            Span::call_site(),
            "#[derive(AerroHandler)] only works on structs",
        )
        .to_compile_error();
    }

    let attrs = match parse_aerro_attrs(&input) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };

    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let service = attrs.service.unwrap_or_else(|| "unknown".into());
    let rpc = attrs.rpc.unwrap_or_else(|| struct_name.to_string());
    let exposure_ident = attrs.exposure.unwrap_or(ExposureAttr::Internal).ident();
    let max_frames = attrs.max_frames.unwrap_or(16u8);

    quote! {
        impl #impl_generics ::aerro::AerroHandler for #struct_name #ty_generics #where_clause {
            const SERVICE: &'static str = #service;
            const RPC: &'static str = #rpc;
            const EXPOSURE: ::aerro::Exposure = ::aerro::Exposure::#exposure_ident;
            const MAX_FRAMES: u8 = #max_frames;
        }

        impl #impl_generics #struct_name #ty_generics #where_clause {
            #[allow(clippy::result_large_err)]
            pub async fn call_tonic(
                &self,
                req: <Self as ::aerro::Handler>::Request,
            ) -> ::core::result::Result<
                <Self as ::aerro::Handler>::Response,
                ::tonic::Status,
            >
            where
                Self: ::aerro::AerroHandler,
            {
                let __opts = ::aerro::wire::encode::EncodeOptions {
                    exposure: <Self as ::aerro::AerroHandler>::EXPOSURE,
                    max_frames: <Self as ::aerro::AerroHandler>::MAX_FRAMES,
                };
                match <Self as ::aerro::Handler>::handle(self, req).await {
                    ::core::result::Result::Ok(__v) => ::core::result::Result::Ok(__v),
                    ::core::result::Result::Err(__e) => {
                        let mut __sf = ::aerro::ServiceFailure::new(__e);
                        let __cat = ::aerro::Aerro::category(&__sf.inner);
                        let __code = ::aerro::Aerro::code(&__sf.inner);
                        let __msg = ::std::string::ToString::to_string(&__sf.inner);
                        __sf.frames.push(::aerro::Frame::local(
                            <Self as ::aerro::AerroHandler>::SERVICE,
                            <Self as ::aerro::AerroHandler>::RPC,
                            __code,
                            __msg,
                            __cat,
                        ));
                        ::core::result::Result::Err(
                            <::aerro::ServiceFailure<
                                <Self as ::aerro::Handler>::Error,
                            > as ::aerro::IntoStatus>::into_status(__sf, &__opts),
                        )
                    }
                }
            }
        }
    }
}

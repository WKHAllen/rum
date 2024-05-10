use proc_macro::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote, ToTokens};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Block, FnArg, ItemFn, ReturnType, Token};

#[proc_macro_attribute]
pub fn handler(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut fn_item = parse_macro_input!(item as ItemFn);

    let main_crate = match crate_name("rum") {
        Ok(FoundCrate::Name(name)) => {
            let ident = format_ident!("{}", name);
            quote!(::#ident)
        }
        _ => quote!(::rum),
    };

    let body = &fn_item.block;
    let args = fn_item
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(pat) => Some(pat),
        })
        .collect::<Vec<_>>();
    let arg_idents = args
        .iter()
        .map(|&arg| {
            let ident = &arg.pat;

            quote! {
                #ident
            }
        })
        .collect::<Vec<_>>();
    let arg_types = args
        .iter()
        .map(|&arg| {
            let ty = &arg.ty;

            quote! {
                #ty
            }
        })
        .collect::<Vec<_>>();
    let arg_from_requests = args
        .iter()
        .map(|arg| {
            let ty = &arg.ty;

            quote! {
                <#ty as #main_crate::request::FromRequest>::from_request(&req)?
            }
        })
        .collect::<Vec<_>>();
    let return_type = match &fn_item.sig.output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    let new_args = quote! { req: #main_crate::request::ServerRequest };
    let new_block = quote! {
        {
            let ( #(#arg_idents),* ): ( #(#arg_types),* ) = match (move || -> #main_crate::error::Result<( #(#arg_types),* )> {
                Ok(( #(#arg_from_requests),* ))
            })() {
                Ok(args) => args,
                Err(err) => {
                    return #main_crate::response::ServerResponse::new_error(err);
                },
            };
            let res: #return_type = async move #body.await;
            #main_crate::response::IntoResponse::into_response(res)
        }
    }
    .into();
    let new_return = quote! {
        -> #main_crate::response::ServerResponse
    }
    .into();

    fn_item.sig.inputs =
        Parser::parse2(Punctuated::<FnArg, Token![,]>::parse_terminated, new_args).unwrap();
    fn_item.sig.output = parse_macro_input!(new_return as ReturnType);
    fn_item.block = Box::new(parse_macro_input!(new_block as Block));

    fn_item.to_token_stream().into()
}

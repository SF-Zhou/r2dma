use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemTrait, ReturnType, TraitItem};

#[proc_macro_attribute]
pub fn service(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemTrait);

    let trait_ident = &input.ident;
    let visibility = input.vis;
    let trait_name = trait_ident.to_string();

    let mut send_bounds = vec![];
    let mut invoke_branchs = vec![];
    let mut client_methods = vec![];

    let krate = get_crate_name();

    let input_items = input.items;
    for item in &input_items {
        if let TraitItem::Fn(method) = item {
            let inputs = &method.sig.inputs;
            if inputs.len() != 3
                || !matches!(inputs[0], FnArg::Receiver(_))
                || method.sig.asyncness.is_none()
            {
                panic!("the function should be in the form `async fn func(&self, ctx: &Context, req: &Req) -> Result<Rsp>`.");
            }

            let method_ident = &method.sig.ident;
            if *method_ident == "rpc_export" || *method_ident == "rpc_call" {
                panic!("Functions cannot be named `rpc_export` or `rpc_call`!");
            }

            let method_name = format!("{trait_name}/{method_ident}");

            let req_type = if let FnArg::Typed(ty) = &inputs[2] {
                ty.ty.clone()
            } else {
                panic!("third param is not a typed arg.");
            };

            let rsp_type = if let ReturnType::Type(_, ty) = &method.sig.output {
                ty.as_ref().clone()
            } else {
                panic!("return value is not a result type.");
            };

            client_methods.push(quote! {
                async fn #method_ident(&self, ctx: &#krate::Context, req: #req_type) -> #rsp_type {
                    self.rpc_call(ctx, req, #method_name).await
                }
            });

            send_bounds.push(quote! { Self::#method_ident(..): Send, });
            invoke_branchs.push(quote! {
                let this = self.clone();
                map.insert(
                    #method_name.into(),
                    Box::new(move |ctx, mut msg| {
                        let this = this.clone();
                        tokio::spawn(async move {
                            match msg.deserialize_payload() {
                                Ok(req) => {
                                    let result = this.#method_ident(&ctx, &req).await;
                                    ctx.send_rsp(msg.meta, result).await;
                                }
                                Err(e) => {
                                    ctx.send_rsp::<(), #krate::Error>(msg.meta, Err(e)).await;
                                }
                            }
                        });
                        Ok(())
                    }),
                );
            });
        } else {
            panic!("only function interfaces are allowed to be defined.");
        }
    }

    quote! {
        #visibility trait #trait_ident {
            const NAME: &'static str = #trait_name;

            #(#input_items)*

            fn rpc_export(
                self: ::std::sync::Arc<Self>,
            ) -> ::std::collections::HashMap<String, #krate::Method>
            where
                Self: 'static + Send + Sync,
                #(#send_bounds)*
            {
                let mut map = ::std::collections::HashMap::<String, #krate::Method>::default();
                #(#invoke_branchs)*
                map
            }
        }

        impl #trait_ident for #krate::Client {
            #(#client_methods)*
        }
    }
    .into()
}

pub(crate) fn get_crate_name() -> proc_macro2::TokenStream {
    let found_crate = proc_macro_crate::crate_name("r2pc").unwrap_or_else(|err| {
        eprintln!("Warning: {err}\n    => defaulting to `crate`",);
        proc_macro_crate::FoundCrate::Itself
    });

    match found_crate {
        proc_macro_crate::FoundCrate::Itself => quote! { crate },
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote! { ::#ident }
        }
    }
}

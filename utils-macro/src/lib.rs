use args::{Args, Command};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ItemFn};
use syn::{Expr, Ident};

use crate::args::Resource;

mod args;

#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    let item_fn = parse_macro_input!(item as ItemFn);
    if item_fn.sig.asyncness.is_none() {
        panic!("Function should be async!");
    }
    let func_ident = &item_fn.sig.ident;
    let (name, description) = {
        if args.len() == 1 {
            (
                func_ident.to_string(),
                args::get_string_value(&args[0], "description"),
            )
        } else if args.len() == 2 {
            (
                args::get_string_value(&args[0], "name"),
                args::get_string_value(&args[1], "description"),
            )
        } else {
            panic!("Expected correct arguments number. Use `name = \"...\"`, `description = \"...\"` or only description")
        }
    };
    let varname = Command::get_varname(func_ident);
    let block = item_fn.block;
    TokenStream::from(quote! {
        static #varname: Command = Command {
            name: #name,
            description: #description,
            func: #func_ident,
        };
        pub fn #func_ident<'a>(sp: Arc<LauncherServiceProvider>, args: &'a [&str]) -> BoxFuture<'a, ()> {
            async move
                #block
            .boxed()
        }
    })
}

#[proc_macro]
pub fn register_commands(item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(item as Args<Ident>);
    let mut stream = TokenStream::new();
    for ident in args.vars {
        let varname = Command::get_varname(&ident);
        stream.extend(TokenStream::from(quote! {
            helper.new_command(&#varname);
        }));
    }
    stream
}

#[proc_macro]
pub fn hash(item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(item as Args<Expr>);
    let mut stream = TokenStream::new();
    let hash_args = args.vars.get(0).expect("Failed to get args value!");
    for expr in args.vars.iter().skip(1) {
        if let Expr::MethodCall(expr) = expr {
            let resource_name = Resource::get_name(&expr.method);
            stream.extend(TokenStream::from(quote! {
                let resource_name = #resource_name;
                if #hash_args.is_empty() || #hash_args.contains(&resource_name) {
                    #expr.await;
                    info!("Successfully rehash {}!", resource_name);
                }
            }));
        }
    }
    stream
}

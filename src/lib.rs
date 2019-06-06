extern crate proc_macro;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

// use crate::proc_macro::TokenStream;
use quote::quote;
// use syn;
use syn::DeriveInput;

use proc_macro::TokenStream;
use syn::{braced, parse_macro_input, token, Field, Ident, LitStr, Result, Token};

use syn::parse::{Parse, ParseStream, Result as ParseResult};

struct Foo {
    name: syn::Ident,
    path: String,
}

impl Parse for Foo {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let name = input.parse::<Ident>()?;
        let path = input.parse::<LitStr>()?.value();

        Ok(Foo { name, path })
    }
}

#[proc_macro]
pub fn make_get_router(tokens: TokenStream) -> TokenStream {
    use crate::proc_macro::TokenStream;
    use syn::parse::Parser;
    use syn::punctuated::Punctuated;
    use syn::Item;

    let parser = Punctuated::<Foo, Token![,]>::parse_terminated;
    let args = parser.parse(tokens).unwrap();

    let def_start = "pub fn get_router() -> router::Router { use router::Router; let mut router = Router::new();";
    let def_end = " router }";

    let mut def_acc = "".to_string();

    for a in args {
        let n = a.name.to_string();
        let p = a.path;
        def_acc.push_str(&format!("#[path=\"{}.rs\"] mod {};", &n, &n));
        def_acc.push_str(&format!(
            "router.get(\"{}\", {}::PageState::handler, \"{}\");",
            &p, &n, &p
        ));
        def_acc.push_str(&format!(
            "router.post(\"{}\", {}::PageState::handler, \"{}\");",
            &p, &n, &p
        ));
    }
    let full_def = format!("{}{}{}", def_start, def_acc, def_end);
    full_def.parse().unwrap()
}

#[proc_macro_derive(RspTraits, attributes(html, table, field))]
pub fn rsp_traits_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use crate::proc_macro::TokenStream;
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let expanded = quote! {
       impl RspStateName for #name {
          fn get_template_name() -> String {
             // stringify!(#name).to_string()
             module_path!().split("::").last().unwrap().to_string()
          }
       }
    };
    TokenStream::from(expanded)
}

#[proc_macro_derive(RspHandlers, attributes(html, html_fill, html_hook, table, field))]
pub fn rsp_handlers_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use crate::proc_macro::TokenStream;

    let ast = parse_macro_input!(input as DeriveInput);

    impl_handlers(&ast)
}

fn get_field_attr<'a>(name: &str, v: &'a Vec<syn::Attribute>) -> Option<&'a syn::Attribute> {
    if v.len() > 0 {
        for attr in v {
            let type_name = &attr
                .path
                .segments
                .pairs()
                .last()
                .unwrap()
                .into_value()
                .ident;
            if type_name == name {
                return Some(&attr);
            }
        }
        return None;
    } else {
        return None;
    }
}

enum FieldFlavour {
    Vector,
    Check,
    Button,
    Select,
    Text,
    Hidden,
}

#[derive(PartialEq, Debug, Clone)]
struct FieldInfo {
    field_name: String,
    field_type: String,
    field_subtype: String,
    html_fill: String,
    html_hook: String,
}

fn get_field_flavour(fi: &FieldInfo) -> FieldFlavour {
    if fi.field_type == "Vec" {
        return FieldFlavour::Vector;
    } else if fi.field_name.starts_with("cb") {
        return FieldFlavour::Check;
    } else if fi.field_name.starts_with("txt") {
        return FieldFlavour::Text;
    } else if fi.field_name.starts_with("btn") {
        return FieldFlavour::Button;
    } else if fi.field_name.starts_with("dd") {
        return FieldFlavour::Select;
    }
    return FieldFlavour::Hidden;
}

fn gen_html_inputs_rec_def(name: &str, field_infos: &Vec<FieldInfo>) -> String {
    let is_toplevel = name == "PageState";
    let mut def_acc = "".to_string();
    let def_start = format!(
        "\n#[derive(Default, Debug, Clone)]\npub struct html_elements_{} {{\n",
        name
    );
    let def_end = format!("\n}}\n");
    for fi in field_infos {
        let field_rec: String = match get_field_flavour(&fi) {
            FieldFlavour::Vector => format!("   pub {}: Vec<html_elements_{}>,\n", &fi.field_name, &fi.field_subtype),
            FieldFlavour::Check => format!("   pub {}: std::rc::Rc<std::cell::RefCell<HtmlCheck>>,\n", &fi.field_name),
            FieldFlavour::Text => format!("   pub {}: std::rc::Rc<std::cell::RefCell<HtmlText>>,\n", &fi.field_name),
            FieldFlavour::Button => format!("   pub {}: std::rc::Rc<std::cell::RefCell<HtmlButton>>,\n", &fi.field_name),
            FieldFlavour::Select => format!("   pub {}: std::rc::Rc<std::cell::RefCell<HtmlSelect<{}>>>,\n", &fi.field_name, &fi.field_type),
            x => format!("/* unsupported {fname} */ pub {fname}: std::rc::Rc<std::cell::RefCell<HtmlText>>,\n", fname=&fi.field_name),
        };
        def_acc.push_str(&field_rec);
    }
    format!("{}\n{}\n{}", def_start, def_acc, def_end)
}

fn gen_html_inputs_rec_init(name: &str, field_infos: &Vec<FieldInfo>) -> String {
    let is_toplevel = name == "PageState";
    let mut def_acc = "".to_string();
    let def_start = format!("html_elements_{} {{\n", name);
    let def_end = format!("\n}}\n");
    for fi in field_infos {
        let field_rec: String = match get_field_flavour(&fi) {
            FieldFlavour::Vector => format!("   {},\n", &fi.field_name),
            FieldFlavour::Check => format!("   {},\n", &fi.field_name),
            FieldFlavour::Text => format!("   {},\n", &fi.field_name),
            FieldFlavour::Button => format!("   {},\n", &fi.field_name),
            FieldFlavour::Select => format!("   {},\n", &fi.field_name),
            x => format!(
                "/* unsupported {fname} */  {fname},\n",
                fname = &fi.field_name
            ),
        };
        def_acc.push_str(&field_rec);
    }
    format!("{}\n{}\n{}", def_start, def_acc, def_end)
}

fn gen_derived_macro_def(name: &str, field_infos: &Vec<FieldInfo>) -> String {
    let is_toplevel = name == "PageState";

    let start_trailer = if is_toplevel {
        "".to_string()
    } else {
        format!("_{}", name)
    };

    let def_start_args_top =
        "{ ($gd: ident, $key: ident, $state: ident, $default_state: ident, $modified: ident) => { {";
    let def_start_args_sub = "{ ($gd: ident, $field: ident, $i: expr, $state: ident, $default_state: ident, $modified: ident) => { {";

    let def_start_args = if is_toplevel {
        def_start_args_top
    } else {
        def_start_args_sub
    };
    let def_start = format!(
        "\n\nmacro_rules! derived_html_inputs{name_trailer} {start_args}",
        name_trailer = &start_trailer,
        start_args = def_start_args
    );
    let def_end = "} }; }";
    let mut def_acc = "".to_string();

    for fi in field_infos {
        match get_field_flavour(&fi) {
            FieldFlavour::Vector => {
                let vec_acc = format!("
                    let mut html_forms: HtmlFormVector = HtmlFormVector::new(stringify!({fname}));
                    let mut {fname}: Vec<html_elements_{sub_type}> = vec![];
                    for i in 0..$state.{fname}.len() {{
                        let mut gd = HtmlForm::new();
                        {fname}.push(derived_html_inputs_{sub_type}!(gd, {fname}, i, $state, $default_state, $modified));

                        html_forms.forms.push(gd);
                    }}
                    $gd.push(std::rc::Rc::new(std::cell::RefCell::new(html_forms)));
                    /* {ftype} */
                    ", fname=&fi.field_name, ftype=&fi.field_type, sub_type=&fi.field_subtype);
                def_acc.push_str(&vec_acc);
                def_acc.push_str(&format!(
                    "/* X embedded {} -> {} */\n",
                    &fi.field_name, &fi.field_type
                ));
            }
            FieldFlavour::Check => {
                if is_toplevel {
                    def_acc.push_str(&format!(
                        "html_check!($gd, {}, $state, $default_state, $modified);\n",
                        &fi.field_name
                    ));
                } else {
                    def_acc.push_str(&format!(
                    "html_nested_check!($gd, $field, $i, {}, $state, $default_state, $modified);\n",
                    &fi.field_name
                ));
                }
            }
            FieldFlavour::Text => {
                if is_toplevel {
                    def_acc.push_str(&format!(
                        "html_text!($gd, {}, $state, $default_state, $modified);\n",
                        &fi.field_name
                    ));
                } else {
                    def_acc.push_str(&format!(
                    "html_nested_text!($gd, $field, $i, {}, $state, $default_state, $modified);\n",
                    &fi.field_name
                ));
                }
            }
            FieldFlavour::Button => {
                if &fi.html_fill != "" {
                    def_acc.push_str(&format!(
                        "html_button!($gd, {}, {});\n",
                        &fi.field_name, &fi.html_fill
                    ));
                } else {
                    def_acc.push_str(&format!(
                        "html_button!($gd, {}, {});\n",
                        &fi.field_name, &fi.field_name
                    ));
                }
            }
            FieldFlavour::Select => {
                if &fi.html_fill != "" {
                    if is_toplevel {
                        def_acc.push_str(&format!(
                            "html_select!($gd, {}, {}, $state, $default_state, $modified);\n",
                            &fi.field_name, &fi.html_fill
                        ));
                    } else {
                        def_acc.push_str(&format!(
                                "html_nested_select!($gd, $field, $i, {}, {}, $state, $default_state, $modified);\n",
                                &fi.field_name, &fi.html_fill
                            ));
                    }
                } else {
                    panic!(
                        "field {} is dropdown, requires 'html_fill' attribute",
                        &fi.field_name
                    );
                }
            }
            _ => {
                def_acc.push_str(&format!("/* unhandled {} */\n", &fi.field_name));
                if is_toplevel {
                    def_acc.push_str(&format!(
                        "html_text!($gd, {}, $state, $default_state, $modified);\n",
                        &fi.field_name
                    ));
                } else {
                    def_acc.push_str(&format!(
                    "html_nested_text!($gd, $field, $i, {}, $state, $default_state, $modified);\n",
                    &fi.field_name
                ));
                }
            }
        }
    }
    def_acc.push_str(&gen_html_inputs_rec_init(name, field_infos));
    format!("{}\n{}\n{}", def_start, def_acc, def_end)
}

fn impl_handlers(ast: &syn::DeriveInput) -> TokenStream {
    let mut field_infos: Vec<FieldInfo> = vec![];

    let name = &ast.ident;
    let name_str = format!("{}", &name);
    let is_toplevel = name == "PageState";
    let mut def_acc = "".to_string();

    if let syn::Data::Struct(datastruct) = &ast.data {
        if let syn::Fields::Named(fieldsnamed) = &datastruct.fields {
            let mut out_debug: Vec<String> = vec![];

            let nfields = fieldsnamed.named.len();
            out_debug.push(format!("total fields: {}", &nfields));

            for field in fieldsnamed.named.iter() {
                let html_hook = if let Some(attr) = get_field_attr("html_hook", &field.attrs) {
                    format!("{};", &attr.tts)
                } else {
                    format!("")
                };
                // def_acc.push_str(&html_hook);
                let html_fill = if let Some(attr) = get_field_attr("html_fill", &field.attrs) {
                    format!("{}", &attr.tts)
                } else {
                    format!("")
                };

                let field_name = if let Some(ident) = &field.ident {
                    format!("{}", &ident)
                } else {
                    panic!("Could not determine field name");
                };
                let html_field_name = format!("html_{}", &field_name);
                let mut inner_type_name = format!("");
                let field_type = if let syn::Type::Path(tp) = &field.ty {
                    // use quote::ToTokens;
                    if tp.path.segments.len() > 1 {
                        panic!("Only simple (path len = 1) types are supported)");
                    }
                    if let Some(ref qself) = tp.qself {
                        if let syn::Type::Path(ref self_type_path) = *qself.ty {
                            let self_type_name =
                                &tp.path.segments.pairs().last().unwrap().into_value().ident;
                            println!("self type: {}", &self_type_name);
                        }
                    }
                    let type_node = &tp.path.segments.pairs().last().unwrap().into_value();
                    // let type_name = &tp.path.segments.pairs().last().unwrap().into_value().ident;
                    let type_name = type_node.ident.clone();

                    match &type_node.arguments {
                        syn::PathArguments::None => {
                            println!("no arguments for {}", &type_name);
                        }
                        syn::PathArguments::AngleBracketed(aba) => {
                            println!("Angle bracketed for {}", &type_name);
                            let arg = aba.args.first().unwrap().into_value();
                            if let syn::GenericArgument::Type(ty) = arg {
                                if let syn::Type::Path(itp) = &ty {
                                    let type_node2 =
                                        &itp.path.segments.pairs().last().unwrap().into_value();
                                    let type_name2 = type_node2.ident.clone();
                                    inner_type_name = format!("{}", &type_name2);
                                    println!("second type name: {}", &type_name2);
                                }
                            }
                        }
                        syn::PathArguments::Parenthesized(para) => {
                            println!("Parenthesized for {}", &type_name);
                        }
                    }
                    format!("{}", &type_name)
                } else {
                    panic!("Only simple types (path, path len = 1) are supported");
                };
                let finfo = FieldInfo {
                    field_name: field_name.clone(),
                    field_type: field_type.clone(),
                    field_subtype: inner_type_name.clone(),
                    html_hook: html_hook.clone(),
                    html_fill: html_fill.clone(),
                };
                field_infos.push(finfo);

                out_debug.push(format!(" {} : {}", &field_name, &field_type));
            }

            println!("FIELD_INFOS: {:#?}", &field_infos);

            println!("{:#?}", &out_debug);
            def_acc.push_str(&gen_html_inputs_rec_def(&name_str, &field_infos));
            def_acc.push_str(&gen_derived_macro_def(&name_str, &field_infos));
            let full_def = format!("{}", &def_acc);
            println!("full def: {}", &full_def);
            full_def.parse().unwrap()
        } else {
            panic!("Can not derive on unnamed fields");
        }
    } else {
        panic!("Can not derive RspHandlers on non-struct");
    }
}

#[proc_macro_derive(RspXHandlers, attributes(html, table, field))]
pub fn rsp_xhandlers_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let mut ast = syn::parse(input).unwrap();
    let mut output = proc_macro::TokenStream::new();

    // Build the trait implementation
    let gen: proc_macro2::TokenStream = impl_hello_macro(&mut ast).into();

    // let ref m = vec![1,2,3,4];
    let ref m = vec![1, 2, 3, 4];
    let mx = vec!["test1", "test2", "test3", "test4"];
    let mx_ident: Vec<syn::Expr> = mx
        .into_iter()
        .map(|x| syn::parse_str(&format!("id_{}", x)).unwrap())
        .collect();
    // let m1 = mx;
    // let x: syn::Type = syn::parse_str("std::collections::HashMap<String, Value>").unwrap();
    let x: syn::Expr = syn::parse_str("i32").unwrap();

    // let x_i32: syn::Type = syn::parse_str("i32").unwrap();

    let mut output2: proc_macro::TokenStream = quote! {
       fn Test() {
          #(let #mx_ident  = #m;)*
       }
    }
    .into();

    let ref struct_field_names = vec!["ddTestValue", "txtAnotherValue"];
    let ref struct_field_types = vec!["i32", "String"];
    let sf_names: Vec<syn::Expr> = struct_field_names
        .into_iter()
        .map(|x| syn::parse_str(&format!("{}", x)).unwrap())
        .collect();
    let sf_types: Vec<syn::Expr> = struct_field_types
        .into_iter()
        .map(|x| syn::parse_str(&format!("{}", x)).unwrap())
        .collect();
    let hhh: syn::Expr = syn::parse_str("html_input(test, test)").unwrap();

    output.extend(output2);
    // panic!("{:#?}", output);
    output.into()
}
fn impl_hello_macro(ast: &mut syn::DeriveInput) -> proc_macro::TokenStream {
    let name = &ast.ident;
    let mut fields_acc: Vec<(String, String)> = vec![];

    match &ast.data {
        syn::Data::Struct(ref a_struct) => {
            match &a_struct.fields {
                syn::Fields::Named(ref a_fields) => {
                    // a_fields.named.pairs().for_each(|x| panic!("{:?}", &x.into_value().ident));
                    parse_fields(a_fields);
                }
                _ => {
                    panic!("rspten: struct should contain only named fields");
                }
            }
        }
        _ => {
            panic!("not a struct");
        }
    };
    let gen = quote! {
        // impl HelloMacro for #name {
            fn handler() {
                // println!("Hello, Macro! My name is {}", stringify!(#name));
            }
        // }
    };
    gen.into()
}

fn parse_fields(f: &syn::FieldsNamed) {
    use quote::ToTokens;

    let mut dd: Vec<String> = vec![];
    f.named.pairs().for_each(|x| {
        // let () = x.into_value();
        let fld = x.into_value();
        dd.push(format!("{:?}", &fld.ident));
        for attr in &fld.attrs {
            // dd.push(format!("    attr {:?}", &attr.tts));
            let mm = attr.interpret_meta();
            if let Some(mm_val) = mm {
                dd.push(format!("    attr {:?}", &mm_val.name()));
            }
        }
        match &fld.ty {
            syn::Type::Verbatim(vtype) => {
                dd.push(format!("{:?}", &vtype.tts));
            }
            syn::Type::Path(vtype) => {
                dd.push("-path-".to_string());
                // dd.push(format!("path {:?}", &vtype.into_token_stream()));
                dd.push(format!("path {:?}", &vtype.into_token_stream()));
                dd.push(format!("len: {}", vtype.path.segments.len()));
                vtype.path.segments.pairs().for_each(|tp| {
                    dd.push(format!("{}", &tp.into_value().ident));
                });
            }
            _ => {
                dd.push("--".to_string());
            }
        }
    });
    println!("{:#?}", dd);
    // panic!("{:#?}", dd);
}

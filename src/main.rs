extern crate clap;
extern crate proc_macro2;
extern crate syn;

use std::collections::HashMap;
use std::fs;

use clap::{App, Arg};
use proc_macro2::LineColumn;
use proc_macro2::Span;
use syn::{
    punctuated::Punctuated, token::Or, visit, Ident, ImplItemMethod, ItemFn, Local, Pat, Type,
};

pub struct Case {
    loc: usize,
    // violates_type: bool,
    is_original: bool,
}

impl std::fmt::Debug for Case {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.loc)
    }
}

impl Case {
    fn new(loc: usize, is_original: bool) -> Self {
        Case { loc, is_original }
    }
}

// #[derive(Default)]
pub struct Count {
    // num: isize,
    locs: Vec<Case>,
    // prev_type: Option<Type>,
}

impl Count {
    fn new() -> Self {
        Count {
            // num: n,
            locs: Vec::new(),
            // prev_type: None,
        }
    }

    // fn from_type(t: Type) -> Self {

    // }
}

// #[derive(Default)]
pub struct Function {
    name: String,
    loc: usize,
    vars: HashMap<Ident, Count>,
    has_shadow: bool,
}

// #[derive(Debug, Clone, Default)]
// pub struct LocalFold {
//     idents: Vec<Ident>,
// }

impl Function {
    fn new(name: String, loc: usize) -> Self {
        Function {
            name,
            loc,
            vars: HashMap::new(),
            has_shadow: false,
        }
    }
}

impl std::fmt::Display for Function {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", 5)
    }
}

// #[derive(Default)]
pub struct ShadowCounter {
    funcs: Vec<Function>,
    // max_len: usize,
    // current_func: Option<Ident>,
}

impl ShadowCounter {
    fn new() -> Self {
        ShadowCounter {
            funcs: Vec::new(),
            // current_func: None,
        }
    }
}
pub fn get_idents(pattern: &Punctuated<Pat, Or>) -> Vec<Ident> {
    let mut idents = Vec::<Ident>::new();
    for p in pattern {
        match p {
            Pat::Ident(i) => {
                if i.by_ref.is_none() {
                    idents.push(i.ident.clone());
                }
            }
            _ => continue,
        }
    }
    return idents;
}

impl<'ast> visit::Visit<'ast> for ShadowCounter {
    fn visit_item_fn(&mut self, i: &ItemFn) {
        // println!("{}", i.ident.to_string());
        self.funcs.push(Function::new(
            i.ident.to_string(),
            i.ident.span().start().line,
        ));
        // self.current_func = i.ident.clone();
        visit::visit_item_fn(self, i);
    }

    fn visit_impl_item_method(&mut self, i: &'ast ImplItemMethod) {
        // println!("{}", i.sig.ident.to_string());
        self.funcs.push(Function::new(
            i.sig.ident.to_string(),
            i.sig.ident.span().start().line,
        ));
        // self.current_func = i.ident.clone();
        visit::visit_impl_item_method(self, i);
    }

    fn visit_local(&mut self, i: &Local) {
        // println!("{:?}", i);

        let ids = get_idents(&i.pats);
        {
            let func_counter: Option<&mut Function> = self.funcs.last_mut();
            // .expect("Cannot have a local without a function.");

            if func_counter.is_none() {
                panic!(
                    "Local without a function? line: {}",
                    ids.get(0).unwrap().span().start().line
                );
            }

            let func_counter = func_counter.unwrap(); // Guarenteed to not crash here.

            for i in ids {
                let line = i.span().start().line;
                let count = func_counter.vars.entry(i).or_insert(Count::new());

                let is_original: bool = count.locs.len() == 0;
                count.locs.push(Case::new(line, is_original));

                if !is_original {
                    func_counter.has_shadow = true;
                }

                // if let Some((_, ty)) = i.ty {}
            }
        }

        visit::visit_local(self, i);
        // self
    }
}

fn print_counter(counter: ShadowCounter) {
    let funcs = counter.funcs;
    // let funcs = counter.funcs;

    for f in funcs {
        if f.has_shadow {
            let vars = f.vars;
            println!(
                "  {:>3}, {:<15} num vars: {}",
                f.loc,
                format!("{} ->", f.name),
                vars.len()
            );

            for (key, val) in vars.iter() {
                if val.locs.len() == 1 {
                    println!("    {:<15.15}>     X", key.to_string());
                } else {
                    println!(
                        "    {:<15.15}> {:5} @ {:?}",
                        key.to_string(),
                        val.locs.len(),
                        val.locs
                    );
                }
            }

            println!();
        }
    }
}

fn main() {
    let matches = App::new("cargo-light")
        .about("Finds and prints potential usages of variable shadowing.")
        .arg(
            Arg::with_name("files")
                .required(true)
                .takes_value(true)
                .multiple(true)
                .help("Files to be checked."),
        )
        .get_matches();

    if let Some(files) = matches.values_of("files") {
        for file in files {
            println!("reading {}", file);
            let source = fs::read_to_string(file).unwrap();
            let syntax = syn::parse_file(&source).expect("Unable to parse file");
            let mut visitor = ShadowCounter::new();
            visit::visit_file(&mut visitor, &syntax);
            print_counter(visitor);
        }
    }
    // println!("{:#?}", syntax);
}

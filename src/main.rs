extern crate clap;
extern crate colored;
extern crate proc_macro2;
extern crate syn;
extern crate walkdir;

use clap::{App, Arg, SubCommand};
use colored::Colorize;
use syn::{punctuated::Punctuated, token::Or, visit, Ident, ImplItemMethod, ItemFn, Local, Pat};
use walkdir::{DirEntry, WalkDir};

use std::collections::HashMap;
use std::fs;

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Case {
    loc: usize,
    // violates_type: bool,
    _is_original: bool,
}

impl std::fmt::Debug for Case {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.loc.to_string().yellow())
    }
}

impl Case {
    fn new(loc: usize, _is_original: bool) -> Self {
        Case { loc, _is_original }
    }
}

#[derive(Default, Clone)]
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

#[derive(Default, Clone)]
pub struct Function {
    name: String,
    loc: usize,
    vars: HashMap<Ident, Count>,
    has_shadow: bool,
}

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
        let vars = &self.vars;
        let head = format!(
            "  {} {:>3} {:<15}",
            "line:".bright_magenta(),
            self.loc.to_string().bright_magenta(),
            self.name.bright_green()
        );

        let mut functions = String::from("");
        for (key, val) in vars.iter() {
            // if val.locs.len() == 1 {
            // write!(fmt,"    {:<15.15}>     X", key.to_string());
            // } else {
            if val.locs.len() != 1 {
                functions += &format!(
                    "    {:<15.15} {:>5} {} {:?}\n",
                    key.to_string().bright_white().bold(),
                    val.locs.len().to_string().bright_cyan(),
                    "@".dimmed(),
                    val.locs
                );
            }
        }

        write!(fmt, "{}\n{}", head, functions)

        // fmt.
    }
}

// #[derive(Default)]
pub struct ShadowCounter<'a> {
    funcs: Vec<Function>,
    filename: &'a str,
    has_shadow: bool,
    // max_len: usize,
    // current_func: Option<Ident>,
}

impl<'a> ShadowCounter<'a> {
    fn new(filename: &'a str) -> Self {
        ShadowCounter {
            filename,
            funcs: Vec::new(),
            has_shadow: false,
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

impl<'ast, 'a> visit::Visit<'ast> for ShadowCounter<'a> {
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
                    self.has_shadow = true;
                }

                // if let Some((_, ty)) = i.ty {}
            }
        }

        visit::visit_local(self, i);
        // self
    }
}

fn print_visitor(counter: ShadowCounter) {
    if counter.has_shadow {
        // let funcs = counter.funcs;
        // let funcs = counter.funcs;
        println!("{} contains shadowed variable(s):\n", counter.filename);
        for f in counter.funcs {
            if f.has_shadow {
                println!("{}", f);
            }
        }
    }
}

fn main() {
    let matches = App::new("cargo-light")
        .about("Finds and prints potential usages of shadowed variables.")
        .author("Fisher Darling <fdarlingco@gmail.com")
        .version("0.1.0")
        .bin_name("cargo")
        .subcommand(
            SubCommand::with_name("light")
                .arg(
                    Arg::with_name("files")
                        .short("F")
                        .long("files")
                        // .required_unless("dir")
                        // .conflicts_with("dir")
                        .takes_value(true)
                        .multiple(true)
                        .help("Files to be parsed (can accept a glob)."),
                )
                .arg(
                    Arg::with_name("dir")
                        .short("d")
                        .long("directory")
                        // .conflicts_with("files")
                        .takes_value(true)
                        // .required_unless("files")
                        .multiple(false)
                        // .default_value(".")
                        .help("Directory to walk and parse."),
                ),
        )
        .get_matches();

    println!();

    let mut fc = false;
    let mut dc = false;

    if let Some(files) = matches
        .subcommand_matches("light")
        .unwrap()
        .values_of("files")
    {
        fc = true;
        // println!("Parsing files!");
        for file in files {
            // println!("reading {}", file);
            let source = fs::read_to_string(file).unwrap();
            let syntax = syn::parse_file(&source).expect("Unable to parse file");

            let mut visitor = ShadowCounter::new(file);

            visit::visit_file(&mut visitor, &syntax);
            print_visitor(visitor);
        }
    }
    if let Some(dir) = matches.subcommand_matches("light").unwrap().value_of("dir") {
        dc = true;
        // println!("Parsing dir! {}", dir);
        let walker = WalkDir::new(dir).into_iter();

        for file in walker {
            let file = file.expect("Unable to parse file name.");

            if !is_file_with_ext(&file, "rs") {
                // Not a .rs file
                continue;
            }

            let file = file.path().to_str();
            // println!("{:?}", file);

            if file.is_none() {
                eprintln!("Unable to parse a file.");
                continue;
            }

            let file = file.unwrap();

            let source = fs::read_to_string(file).unwrap();
            let syntax = syn::parse_file(&source).expect("Unable to parse file");

            let mut visitor = ShadowCounter::new(file);

            visit::visit_file(&mut visitor, &syntax);
            print_visitor(visitor);
        }
    }

    if !(fc || dc) {
        let walker = WalkDir::new(".").into_iter();

        for file in walker {
            let file = file.expect("Unable to parse file name.");

            if !is_file_with_ext(&file, "rs") {
                // Not a .rs file
                continue;
            }

            let file = file.path().to_str();
            // println!("{:?}", file);

            if file.is_none() {
                eprintln!("Unable to parse a file.");
                continue;
            }

            let file = file.unwrap();

            let source = fs::read_to_string(file).unwrap();
            let syntax = syn::parse_file(&source).expect("Unable to parse file");

            let mut visitor = ShadowCounter::new(file);

            visit::visit_file(&mut visitor, &syntax);
            print_visitor(visitor);
        }
    }
    // println!("{:#?}", syntax);
}

// Taken from cargo-geiger
// Copyright (c) 2018 Simon Heath
// Copyright (c) 2015-2016 Steven Fackler
fn is_file_with_ext(entry: &DirEntry, file_ext: &str) -> bool {
    if !entry.file_type().is_file() {
        return false;
    }
    let p = entry.path();
    let ext = match p.extension() {
        Some(e) => e,
        None => return false,
    };
    // to_string_lossy is ok since we only want to match against an ASCII
    // compatible extension and we do not keep the possibly lossy result
    // around.
    ext.to_string_lossy() == file_ext
}

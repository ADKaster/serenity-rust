#![feature(proc_macro_span)]
extern crate proc_macro;

use proc_macro::{TokenStream, TokenTree};
use std::fs::read_to_string;
use std::path::Path;

#[proc_macro]
pub fn ipc_file(input_data: TokenStream) -> TokenStream {
    let mut it = input_data.into_iter();
    let path_item = it.next().unwrap();
    assert!(it.next().is_none());
    if let TokenTree::Literal(lit) = path_item {
        let mut string: String = "::serenity::ipc! {".to_string();
        let base_file_path = lit.span().source_file().path();
        let base_path = base_file_path.parent().unwrap_or(Path::new("."));
        let path_fragment_string = lit.span().source_text().unwrap();
        let path_fragment = Path::new(&path_fragment_string[1..path_fragment_string.len() - 1]);
        let path = base_path.join(path_fragment);
        println!("{:?}", path);
        string.push_str(read_to_string(path).unwrap().as_str());
        string.push_str("}");
        string.parse().unwrap()
    } else {
        panic!("Expected a string literal as the first argument to ipc_file!");
    }
}

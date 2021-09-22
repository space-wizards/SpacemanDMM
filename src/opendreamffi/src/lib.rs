extern crate dreammaker as dm;

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::os::raw::c_char;
use std::path::PathBuf;

use dm::annotation::AnnotationTree;
use dm::indents::IndentProcessor;
use dm::objtree::ObjectTree;
use dm::parser::Parser;
use dm::preprocessor::{DefineMap, Preprocessor};
use dm::{Context, Severity};

#[no_mangle]
pub unsafe extern "C" fn sdmm_parse(
    count_files: i32,
    files: *const *const c_char,
) -> *mut ParseResult {
    let paths: Box<[_]> = (0..count_files)
        .map(|i| {
            CStr::from_ptr(*files.add(i as usize))
                .to_str()
                .expect("Environment path was not valid UTF-8")
        })
        .collect();

    Box::into_raw(Box::new(ParseResult::parse(&paths)))
}

#[no_mangle]
pub unsafe extern "C" fn sdmm_result_free(result: *mut ParseResult) {
    std::mem::drop(Box::from_raw(result))
}

#[no_mangle]
pub unsafe extern "C" fn sdmm_result_get_file_list(result_ptr: *mut ParseResult) -> *const c_char {
    let result = result_ptr.as_mut().unwrap();

    let json = result.get_file_list();

    result.export_string(&json)
}

#[no_mangle]
pub unsafe extern "C" fn sdmm_result_get_diagnostics(
    result_ptr: *mut ParseResult,
) -> *const c_char {
    let result = result_ptr.as_mut().unwrap();

    let json = result.get_diagnostics();

    result.export_string(&json)
}

#[no_mangle]
pub unsafe extern "C" fn sdmm_result_get_type_list(result_ptr: *mut ParseResult) -> *const c_char {
    let result = result_ptr.as_mut().unwrap();

    let json = result.get_type_list();

    result.export_string(&json)
}

#[no_mangle]
pub unsafe extern "C" fn sdmm_result_get_type_info(
    result_ptr: *mut ParseResult,
    path_ptr: *const c_char,
) -> *const c_char {
    let path = CStr::from_ptr(path_ptr)
        .to_str()
        .expect("Path is not valid UTF-8");
    let result = result_ptr.as_mut().unwrap();

    let json = result.get_type_info(path);

    result.export_string(&json)
}

#[no_mangle]
pub unsafe extern "C" fn sdmm_result_get_special_files(
    result_ptr: *mut ParseResult,
) -> *const c_char {
    let result = result_ptr.as_mut().unwrap();

    let json = result.get_special_files();

    result.export_string(&json)
}

fn json_to_string<T>(val: &T) -> serde_json::Result<String>
where
    T: ?Sized + serde::Serialize,
{
    // Allows me to easily swap everything out to to_string_pretty if I want.
    serde_json::to_string(val)
}

pub struct ParseResult {
    pub context: Context,
    pub obj_tree: ObjectTree,
    pub annotations: AnnotationTree,
    pub export_buffer: Option<CString>,
    pub maps: Box<[PathBuf]>,
    pub scripts: Box<[PathBuf]>,
    pub skins: Box<[PathBuf]>,
}

impl ParseResult {
    /// Last file passed is the environment file. Files are loaded in order.
    pub fn parse(files: &[&str]) -> Self {
        let context = Context::default();
        let mut annotations = AnnotationTree::default();

        let mut preprocessor = Preprocessor::new_with_builtins(
            &context,
            PathBuf::from(files[files.len() - 1]),
            DefineMap::default(),
        )
        .expect("Failed to load environment!");

        for file_path in files.iter().rev().skip(1) {
            let file = File::open(&file_path).expect("Failed to open file!");
            preprocessor
                .push_file(PathBuf::from(file_path), file)
                .unwrap();
        }
        let mut indent_processor = IndentProcessor::new(&context, &mut preprocessor);
        let mut parser = Parser::new(&context, &mut indent_processor);

        parser.annotate_to(&mut annotations);

        let obj_tree = parser.parse_object_tree_without_builtins();

        let maps = preprocessor.maps().iter().map(|p| p.clone()).collect();
        let skins = preprocessor.skins().iter().map(|p| p.clone()).collect();
        let scripts = preprocessor.scripts().iter().map(|p| p.clone()).collect();

        ParseResult {
            context,
            obj_tree,
            annotations,
            export_buffer: None,
            maps,
            skins,
            scripts,
        }
    }

    pub fn get_file_list(&self) -> String {
        let file_list = self.context.file_list();
        let mut files = Vec::new();

        file_list.for_each(|f| files.push(f.to_owned()));

        json_to_string(&files).expect("JSON serialization failed")
    }

    pub fn get_diagnostics(&self) -> String {
        let diagnostics = self.context.errors();
        let filtered: Box<[_]> = diagnostics
            .iter()
            .filter(|e| e.severity() <= Severity::Warning)
            .collect();

        json_to_string(&filtered).expect("JSON serialization failed")
    }

    pub fn get_type_list(&self) -> String {
        let types = self.obj_tree.iter_types();
        let type_names: Vec<_> = types.map(|t| &t.get().path).collect();

        json_to_string(&type_names).expect("JSON serialization failed")
    }

    pub fn get_type_info(&self, path: &str) -> String {
        let the_type = if path == "" || path == "/" {
            self.obj_tree.root()
        } else {
            self.obj_tree.expect(path)
        }
        .get();

        json_to_string(&the_type).expect("JSON serialization failed")
    }

    pub fn get_special_files(&self) -> String {
        let mut value: HashMap<&'static str, &[PathBuf]> = HashMap::new();
        value.insert("maps", &self.maps);
        value.insert("skins", &self.skins);
        value.insert("scripts", &self.scripts);

        json_to_string(&value).expect("JSON serialization failed")
    }

    fn export_string(&mut self, string: &str) -> *const c_char {
        let c_string = CString::new(string).unwrap();
        let ptr = c_string.as_ptr();
        self.export_buffer = Some(c_string);
        ptr
    }
}

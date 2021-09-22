fn main() {
    let args: Vec<_> = std::env::args().collect();

    let dme = &args[1];
    let path = &args[2];

    let result = sdmm_opendream::ParseResult::parse(&[dme]);
    let info = result.get_type_info(path);

    println!("{}", info);
}

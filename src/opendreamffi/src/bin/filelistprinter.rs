fn main() {
    let args: Vec<_> = std::env::args().collect();

    let dme = &args[1];

    let result = sdmm_opendream::ParseResult::parse(&[dme]);
    let file_list = result.context.file_list();

    let mut c = 1;

    file_list.for_each(|f| {
        println!("{}: {:?}", c, f);
        c += 1;
    });

}

// Read-only diagnostic: prints format names/sizes/errors, never clipboard content.
fn main() {
    use clipboard_win::{raw, Clipboard};
    let _clip = Clipboard::new_attempts(10).expect("Clipboard busy");
    for id in raw::EnumFormats::new() {
        let name = raw::format_name_big(id).unwrap_or_default();
        let size = raw::size(id).map(|n| n.get());
        let result = if id == 13 || name == "HTML Format" || name == "Rich Text Format" {
            let mut b = vec![];
            format!("{:?}", raw::get_vec(id, &mut b))
        } else {
            String::new()
        };
        println!("format={id} name={name:?} bytes={size:?} read={result}");
    }
}

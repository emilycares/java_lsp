pub use smol_str;
pub type MyString = smol_str::SmolStr;

pub fn capitalize_first(s: &MyString) -> MyString {
    let mut chars = s.chars();
    match chars.next() {
        None => MyString::default(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            MyString::new(upper + chars.as_str())
        }
    }
}

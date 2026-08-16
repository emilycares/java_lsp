use core::fmt::Debug;
use std::{cmp, mem, path::Path};

pub fn capitalize_first(s: &NuVec) -> NuVec {
    let mut chars = s.to_str().chars();
    match chars.next() {
        None => NuVec::default(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            let mut b = NuVecBuilder::new();
            b.pusha(upper.as_bytes());
            b.pusha(chars.as_str().as_bytes());
            b.finish()
        }
    }
}
/// A [`u8`] with a bunch of niches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[repr(u8)]
pub enum InlineSize {
    _V0 = 0,
    _V1,
    _V2,
    _V3,
    _V4,
    _V5,
    _V6,
    _V7,
    _V8,
    _V9,
    _V10,
    _V11,
    _V12,
    _V13,
    _V14,
    _V15,
    _V16,
    _V17,
    _V18,
    _V19,
    _V20,
    _V21,
    _V22,
    _V23,
}
impl InlineSize {
    /// SAFETY: `value` must be less than or equal to [`INLINE_CAP`]
    #[inline(always)]
    const unsafe fn transmute_from_u8(value: u8) -> Self {
        debug_assert!(value <= InlineSize::_V23 as u8);
        // SAFETY: The caller is responsible to uphold this invariant
        unsafe { mem::transmute::<u8, Self>(value) }
    }
}
const INLINE_CAP: usize = InlineSize::_V23 as usize;
const N_NEWLINES: usize = 32;
const N_SPACES: usize = 128;
const WS: &[u8] = b"\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n                                                                                                                                ";
const EMPTY: [u8; INLINE_CAP] = [0u8; INLINE_CAP];

/// No utf8 Vec
#[derive(Clone, PartialOrd, Ord)]
pub enum NuVec {
    Inline {
        len: InlineSize,
        buf: [u8; INLINE_CAP],
    },
    Static(&'static [u8]),
    Heap(Vec<u8>),
}
impl core::cmp::Eq for NuVec {}
impl core::hash::Hash for NuVec {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}
impl PartialEq<NuVec> for NuVec {
    fn eq(&self, other: &NuVec) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<String> for NuVec {
    fn eq(&self, other: &String) -> bool {
        *self == *other.as_str()
    }
}

impl PartialEq<str> for NuVec {
    fn eq(&self, other: &str) -> bool {
        match self {
            NuVec::Inline { buf, .. } => {
                let bu = &buf[..self.len()];
                bu == other.as_bytes()
            }
            NuVec::Static(items) => *items == other.as_bytes(),
            NuVec::Heap(items) => items.as_slice() == other.as_bytes(),
        }
    }
}
impl PartialEq<&str> for NuVec {
    fn eq(&self, other: &&str) -> bool {
        match self {
            NuVec::Inline { buf, .. } => {
                let bu = &buf[..self.len()];
                bu == other.as_bytes()
            }
            NuVec::Static(items) => *items == other.as_bytes(),
            NuVec::Heap(items) => items.as_slice() == other.as_bytes(),
        }
    }
}

impl std::convert::AsRef<std::path::Path> for NuVec {
    fn as_ref(&self) -> &std::path::Path {
        Path::new(self.to_str())
    }
}

impl Default for NuVec {
    fn default() -> Self {
        Self::Inline {
            len: InlineSize::_V0,
            buf: EMPTY,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NuVecBuilder {
    Inline { len: usize, buf: [u8; INLINE_CAP] },
    Heap(Vec<u8>),
}

impl Default for NuVecBuilder {
    #[inline]
    fn default() -> Self {
        NuVecBuilder::Inline {
            buf: [0; INLINE_CAP],
            len: 0,
        }
    }
}

impl NuVecBuilder {
    /// Creates a new empty [`NuVecBuilder`].
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        NuVecBuilder::Inline {
            buf: [0; INLINE_CAP],
            len: 0,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            NuVecBuilder::Inline { len, .. } => *len,
            NuVecBuilder::Heap(items) => items.len(),
        }
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        match self {
            NuVecBuilder::Inline { len, .. } => *len as u8 == 0,
            NuVecBuilder::Heap(items) => items.is_empty(),
        }
    }

    /// Builds a [`NuVec`] from `self`.
    #[must_use]
    pub fn finish(self) -> NuVec {
        match self {
            NuVecBuilder::Inline { len, buf } => {
                debug_assert!(len <= INLINE_CAP);
                NuVec::Inline {
                    // SAFETY: We know that `value.len` is less than or equal to the maximum value of `InlineSize`
                    len: unsafe { InlineSize::transmute_from_u8(len as u8) },
                    buf,
                }
            }
            NuVecBuilder::Heap(heap) => NuVec::Heap(heap),
        }
    }

    /// Appends the given [`char`] to the end of `self`'s buffer.
    #[inline]
    pub fn push(&mut self, c: u8) {
        match &mut *self {
            NuVecBuilder::Inline { len, buf } => {
                let char_len = 1;
                let new_len = *len + char_len;
                if new_len <= INLINE_CAP {
                    buf[new_len.saturating_sub(1)] = c;
                    *len += char_len;
                } else {
                    let mut heap = Vec::with_capacity(new_len);
                    heap.extend_from_slice(&buf[..*len]);
                    heap.push(c);
                    *self = NuVecBuilder::Heap(heap);
                }
            }
            NuVecBuilder::Heap(h) => h.push(c),
        }
    }

    #[inline]
    pub fn pusha(&mut self, arg: &[u8]) {
        match &mut *self {
            NuVecBuilder::Inline { len, buf } => {
                let alen = arg.len();
                let new_len = *len + alen;
                if new_len <= INLINE_CAP {
                    buf[*len..new_len].copy_from_slice(&arg[..alen]);
                    *len = new_len;
                } else {
                    let mut heap = Vec::with_capacity(new_len);
                    heap.extend_from_slice(&buf[..*len]);
                    heap.extend(arg);
                    *self = NuVecBuilder::Heap(heap);
                }
            }
            NuVecBuilder::Heap(items) => items.extend(arg),
        }
    }

    #[inline]
    pub fn extend(&mut self, arg: &NuVec) {
        let slen = self.len();

        match (&mut *self, arg) {
            (NuVecBuilder::Inline { buf, len }, NuVec::Inline { buf: ibuf, .. }) => {
                let alen = arg.len();
                let new_len = *len + alen;
                if new_len <= INLINE_CAP {
                    buf[*len..new_len].copy_from_slice(&ibuf[..alen]);
                    *len = new_len;
                } else {
                    let mut d = Vec::with_capacity(new_len);
                    d.extend_from_slice(&buf[..slen]);
                    d.extend_from_slice(&ibuf[..alen]);
                    *self = NuVecBuilder::Heap(d)
                }
            }
            (NuVecBuilder::Inline { buf, len }, NuVec::Static(ibuf)) => {
                let alen = arg.len();
                let new_len = *len + arg.len();
                if new_len <= INLINE_CAP {
                    buf[*len..new_len].copy_from_slice(&ibuf[..alen]);
                    *len = new_len;
                } else {
                    let mut d = Vec::with_capacity(new_len);
                    d.extend_from_slice(&buf[..slen]);
                    d.extend_from_slice(&ibuf[..alen]);
                    *self = NuVecBuilder::Heap(d)
                }
            }
            (NuVecBuilder::Inline { buf, .. }, NuVec::Heap(iitems)) => {
                let mut d = Vec::new();
                d.extend_from_slice(&buf[..slen]);
                d.extend(iitems);
                *self = NuVecBuilder::Heap(d)
            }
            (NuVecBuilder::Heap(items), NuVec::Inline { buf, .. }) => {
                let alen = arg.len();
                items.extend_from_slice(&buf[0..alen])
            }
            (NuVecBuilder::Heap(items), NuVec::Static(iitems)) => items.extend(*iitems),
            (NuVecBuilder::Heap(items), NuVec::Heap(iitems)) => items.extend(iitems),
        }
    }
}

impl From<&str> for NuVec {
    fn from(value: &str) -> Self {
        Self::new(value.as_bytes())
    }
}

impl core::fmt::Display for NuVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl Debug for NuVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // match self {
        //     NuVec::Inline { .. } => {
        //         write!(f, "NuVec::Inline(")?;
        //     }
        //     NuVec::Static(..) => {
        //         write!(f, "NuVec::Static(")?;
        //     }
        //     NuVec::Heap(..) => {
        //         write!(f, "NuVec::Heap(")?;
        //     }
        // };
        write!(f, "\"{}\"", self.to_str())
        // write!(f, ")")
    }
}

impl NuVec {
    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            NuVec::Inline { len, .. } => *len as usize,
            NuVec::Static(items) => items.len(),
            NuVec::Heap(items) => items.len(),
        }
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        match self {
            NuVec::Inline { len, .. } => *len as u8 == 0,
            NuVec::Static(items) => items.is_empty(),
            NuVec::Heap(items) => items.is_empty(),
        }
    }

    /// This function tries to create a new Repr::Inline or Repr::Static
    /// If it isn't possible, this function returns None
    #[inline]
    fn new_on_stack<T>(text: T) -> Option<Self>
    where
        T: AsRef<[u8]>,
    {
        let text = text.as_ref();

        let len = text.len();
        if len <= INLINE_CAP {
            let mut buf = [0; INLINE_CAP];
            buf[..len].copy_from_slice(text);
            return Some(NuVec::Inline {
                // SAFETY: We know that `len` is less than or equal to the maximum value of `InlineSize`
                len: unsafe { InlineSize::transmute_from_u8(len as u8) },
                buf,
            });
        }

        if len <= N_NEWLINES + N_SPACES {
            let bytes = text;
            let possible_newline_count = cmp::min(len, N_NEWLINES);
            let newlines = bytes[..possible_newline_count]
                .iter()
                .take_while(|&&b| b == b'\n')
                .count();
            let possible_space_count = len - newlines;
            if possible_space_count <= N_SPACES && bytes[newlines..].iter().all(|&b| b == b' ') {
                let spaces = possible_space_count;
                let substring = &WS[N_NEWLINES - newlines..N_NEWLINES + spaces];
                return Some(NuVec::Static(substring));
            }
        }
        None
    }

    #[inline]
    pub fn new(text: &[u8]) -> Self {
        Self::new_on_stack(text).unwrap_or_else(|| NuVec::Heap(text.to_vec()))
    }

    #[inline]
    pub fn new_static(text: &'static [u8]) -> Self {
        Self::Static(text)
    }

    pub fn to_str(&self) -> &str {
        match self {
            NuVec::Inline { buf, .. } => str::from_utf8(&buf[0..self.len()]).unwrap_or_default(),
            NuVec::Static(items) => str::from_utf8(items).unwrap_or_default(),
            NuVec::Heap(items) => str::from_utf8(items).unwrap_or_default(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            NuVec::Inline { buf, .. } => &buf[0..self.len()],
            NuVec::Static(items) => items,
            NuVec::Heap(items) => items.as_slice(),
        }
    }

    pub fn starts_with(&self, arg: &[u8]) -> bool {
        self.as_bytes().starts_with(arg)
    }

    pub fn trim_start_matches_byte(&self, arg: u8) -> NuVec {
        if self.starts_with(&[arg]) {
            return match self {
                NuVec::Inline { buf, .. } => {
                    let buf = *buf;
                    let c = &buf[1..self.len()];
                    NuVec::new(c)
                }
                NuVec::Static(items) => NuVec::Static(&items[1..]),
                NuVec::Heap(items) => NuVec::new(&items[1..]),
            };
        }
        self.clone()
    }

    pub fn trim_start_matches(&self, arg: &[u8]) -> NuVec {
        if self.starts_with(arg) {
            return match self {
                NuVec::Inline { buf, .. } => {
                    let buf = *buf;
                    let c = &buf[arg.len()..self.len()];
                    NuVec::new(c)
                }
                NuVec::Static(items) => NuVec::new(&items[arg.len()..]),
                NuVec::Heap(items) => NuVec::new(&items[arg.len()..]),
            };
        }
        self.clone()
    }

    pub fn replace_byte(&self, a: u8, rep: u8) -> NuVec {
        match self {
            NuVec::Inline { len, buf } => {
                let mut out = *buf;
                for b in out.iter_mut() {
                    if *b == a {
                        *b = rep;
                    }
                }

                NuVec::Inline {
                    len: *len,
                    buf: out,
                }
            }
            NuVec::Static(items) => {
                if items.contains(&a) {
                    let out = NuVec::new(items);
                    out.replace_byte(a, rep)
                } else {
                    self.clone()
                }
            }
            NuVec::Heap(items) => {
                let mut out = items.clone();
                for b in out.iter_mut() {
                    if *b == a {
                        *b = rep;
                    }
                }
                NuVec::Heap(out)
            }
        }
    }

    pub fn rsplit_once_byte(&self, arg: u8) -> Option<(NuVec, NuVec)> {
        let bytes = self.as_bytes();
        for (e, i) in bytes.iter().enumerate().rev() {
            if i == &arg {
                let a = &bytes[0..e];
                let b = &bytes[e + 1..];
                return Some((NuVec::new(a), NuVec::new(b)));
            }
        }
        None
    }

    pub fn split_once_byte(&self, arg: u8) -> Option<(NuVec, NuVec)> {
        let bytes = self.as_bytes();
        for (e, i) in bytes.iter().enumerate() {
            if i == &arg {
                let a = &bytes[0..e];
                let b = &bytes[e + 1..];
                return Some((NuVec::new(a), NuVec::new(b)));
            }
        }
        None
    }

    pub fn splitn_byte(&self, max: usize, spl: u8) -> Vec<NuVec> {
        let bytes = self.as_bytes();
        let mut m = 0;
        let mut i = 0;
        let mut out = Vec::new();

        for (index, b) in bytes.iter().enumerate() {
            if b == &spl {
                m += 1;
                if m == max {
                    let next = &bytes[i..];
                    out.push(NuVec::new(next));
                    break;
                } else {
                    let next = &bytes[i..index];
                    out.push(NuVec::new(next));
                }
                i = index + 1;
            }
        }
        out
    }

    pub fn to_lowercase(&self) -> NuVec {
        NuVec::Heap(self.as_bytes().to_ascii_lowercase())
    }

    pub fn ends_with(&self, end: &[u8]) -> bool {
        self.as_bytes().ends_with(end)
    }

    pub fn trim_end_matches(&self, end: &[u8]) -> NuVec {
        match self {
            NuVec::Inline { buf, .. } => {
                let content = &buf[0..self.len()];
                if content.ends_with(end) {
                    return NuVec::new(&buf[0..self.len() - end.len()]);
                }
                self.clone()
            }
            NuVec::Static(items) => {
                if items.ends_with(end) {
                    return NuVec::new(&items[0..items.len() - end.len()]);
                }
                self.clone()
            }
            NuVec::Heap(items) => {
                if items.ends_with(end) {
                    return NuVec::new(&items[0..items.len() - end.len()]);
                }
                self.clone()
            }
        }
    }

    pub fn trim_end_matches_byte(&self, arg: u8) -> NuVec {
        match self {
            NuVec::Inline { buf, .. } => {
                let content = &buf[self.len().saturating_sub(1)..self.len()];
                if content.ends_with(&[arg]) {
                    return NuVec::new(&buf[0..self.len() - 1]);
                }
                self.clone()
            }
            NuVec::Static(items) => {
                if items.ends_with(&[arg]) {
                    return NuVec::new(&items[0..items.len() - 1]);
                }
                self.clone()
            }
            NuVec::Heap(items) => {
                if items.ends_with(&[arg]) {
                    return NuVec::new(&items[0..items.len() - 1]);
                }
                self.clone()
            }
        }
    }

    pub fn contains(&self, arg: &[u8]) -> bool {
        self.as_bytes().windows(arg.len()).any(|i| i == arg)
    }

    pub fn contains_byte(&self, arg: u8) -> bool {
        self.as_bytes().contains(&arg)
    }

    pub fn replace(&self, a: &[u8], b: &[u8]) -> NuVec {
        self.replacen(a, b, usize::MAX)
    }

    pub fn replacen(&self, a: &[u8], b: &[u8], amount: usize) -> NuVec {
        let bytes = self.as_bytes();
        let mut out = NuVecBuilder::new();

        if a.is_empty() || amount == 0 {
            out.pusha(bytes);
            return out.finish();
        }

        let mut replaced = 0usize;
        let mut rest = bytes;

        while replaced < amount {
            match find_subslice(rest, a) {
                Some(pos) => {
                    out.pusha(&rest[..pos]);
                    out.pusha(b);
                    rest = &rest[pos + a.len()..];
                    replaced += 1;
                }
                None => break,
            }
        }

        out.pusha(rest);
        out.finish()
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for NuVec {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let string = <&str>::arbitrary(u)?;
        Ok(NuVec::new(string.as_bytes()))
        // let string = <&[u8]>::arbitrary(u)?;
        // Ok(NuVec::new(string))
    }
}

fn find_subslice(bytes: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > bytes.len() {
        return None;
    }
    bytes
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use crate::{NuVec, NuVecBuilder};

    #[test]
    fn push() {
        let mut b = NuVecBuilder::new();
        b.push(b's');
        b.pusha(b"tart");
        let o = b.finish();
        let o = format!("{}", o);

        let expected = expect![[r#"
            "start"
        "#]];
        expected.assert_debug_eq(&o);
    }

    #[test]
    fn push_long() {
        let mut b = NuVecBuilder::new();
        b.pusha(b"aaaaaaaaaaaaaaaaaaaaaaa");
        b.push(b's');
        b.pusha(b"tart");
        let o = b.finish();
        let o = format!("{}", o);

        let expected = expect![[r#"
            "aaaaaaaaaaaaaaaaaaaaaaastart"
        "#]];
        expected.assert_debug_eq(&o);
    }

    #[test]
    fn extend() {
        let mut s = NuVecBuilder::new();
        s.push(b's');
        let s = s.finish();
        let mut b = NuVecBuilder::new();
        b.pusha(b"tart");
        let b = b.finish();

        let mut o = NuVecBuilder::new();
        o.extend(&s);
        o.extend(&b);
        let o = o.finish();
        let o = format!("{}", o);

        let expected = expect![[r#"
            "start"
        "#]];
        expected.assert_debug_eq(&o);
    }

    #[test]
    fn extend_static() {
        let s = NuVec::new_static(b"s");
        let b = NuVec::new_static(b"tart");

        let mut o = NuVecBuilder::new();
        o.extend(&s);
        o.extend(&b);
        let o = o.finish();
        let o = format!("{}", o);

        let expected = expect![[r#"
            "start"
        "#]];
        expected.assert_debug_eq(&o);
    }

    #[test]
    fn extend_heap() {
        let s = NuVec::Heap(b"s".to_vec());
        let b = NuVec::Heap(b"tart".to_vec());

        let mut o = NuVecBuilder::new();
        o.extend(&s);
        o.extend(&b);
        let o = o.finish();
        let o = format!("{}", o);

        let expected = expect![[r#"
            "start"
        "#]];
        expected.assert_debug_eq(&o);
    }

    #[test]
    fn extend_long() {
        let mut p = NuVecBuilder::new();
        p.pusha(b"aaaaaaaaaaaaaaaaaaaaaaa");
        let p = p.finish();

        let mut s = NuVecBuilder::new();
        s.push(b's');
        let s = s.finish();
        let mut b = NuVecBuilder::new();
        b.pusha(b"tart");
        let b = b.finish();

        let mut o = NuVecBuilder::new();
        o.extend(&p);
        o.extend(&s);
        o.extend(&b);
        let o = o.finish();
        let o = format!("{}", o);

        let expected = expect![[r#"
            "aaaaaaaaaaaaaaaaaaaaaaastart"
        "#]];
        expected.assert_debug_eq(&o);
    }

    #[test]
    fn equal_static_and_builder() {
        let a = NuVec::new_static(b"java.lang.String");
        let mut b = NuVecBuilder::new();
        b.pusha(b"java");
        b.pusha(b".lang");
        b.pusha(b".String");
        let b = b.finish();

        assert_eq!(a, b);
    }

    #[test]
    fn trim_start() {
        let a = NuVec::new_static(b"java.lang.String");
        let b = a.trim_start_matches_byte(b'j');

        assert_eq!(b, NuVec::new_static(b"ava.lang.String"));

        let a = NuVec::new_static(b"java.lang.String");
        let b = a.trim_start_matches(b"java.lang.");

        assert_eq!(b, NuVec::new_static(b"String"));
    }

    #[test]
    fn replace() {
        let a = NuVec::new_static(b"java.lang.String");
        let b = a.replace_byte(b'.', b'/');

        assert_eq!(b, NuVec::new_static(b"java/lang/String"));

        let a = NuVec::new_static(b"java.lang.String");
        let b = a.replace(b"java.lang.", b"avaj.gnal.");

        assert_eq!(b, NuVec::new_static(b"avaj.gnal.String"));

        let a = NuVec::new_static(b"java.lang.String");
        let b = a.replacen(b"java.lang.", b"avaj.gnal.", 1);

        assert_eq!(b, NuVec::new_static(b"avaj.gnal.String"));

        let a = NuVec::new_static(b"java.lang.String");
        let b = a.replacen(b"java.lang.", b"avaj.gnal.", 0);

        assert_eq!(b, NuVec::new_static(b"java.lang.String"));
    }
}

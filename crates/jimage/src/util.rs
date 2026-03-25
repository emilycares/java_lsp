use crate::types::JimageError;

pub type JResult<T> = Result<(usize, T), JimageError>;

#[track_caller]
#[inline]
pub fn expect_data(data: &[u8], pos: usize, expected: &[u8]) -> Result<usize, JimageError> {
    let len = expected.len();
    let Some(get) = data.get(pos..pos + len) else {
        return Err(JimageError::EOF);
    };

    let cond = get != expected;
    if cond {
        #[cfg(debug_assertions)]
        {
            dbg!(get);
            dbg!(expected);
        };
        return Err(JimageError::NotAsExpected { pos, len });
    }
    Ok(pos + len)
}

pub fn get_u8(data: &[u8], pos: usize) -> JResult<u8> {
    let Some(get) = data.get(pos) else {
        return Err(JimageError::EOF);
    };

    Ok((pos + 1, *get))
}

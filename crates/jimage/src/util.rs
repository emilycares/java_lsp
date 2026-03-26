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
pub fn get_u16(data: &[u8], pos: usize) -> JResult<u16> {
    let next = pos + 2;
    let get = <[u8; 2]>::try_from(&data[pos..next]).map_err(JimageError::Number)?;
    let out = u16::from_le_bytes(get);

    Ok((next, out))
}
pub fn get_i32(data: &[u8], pos: usize) -> JResult<i32> {
    let next = pos + 4;
    let get = <[u8; 4]>::try_from(&data[pos..next]).map_err(JimageError::Number)?;
    let out = i32::from_le_bytes(get);

    Ok((next, out))
}

//! 验证码遮挡：系统通知预览里把提取出的验证码打码。

pub fn mask(text: &str, code: &str) -> String {
    if code.is_empty() { return text.into(); }
    // 提取器会归一化全角和零宽字符；遮挡时也匹配原文中的这些写法。
    let pattern = code.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| {
        format!("[{c}{}]", char::from_u32(c as u32 + 0xfee0).unwrap())
    }).collect::<Vec<_>>().join(r"[\s\-\u{ff0d}\u{200b}\u{200c}\u{200d}\u{feff}\u{2060}]*");
    if pattern.is_empty() { return text.replace(code, "••••••"); }
    regex::RegexBuilder::new(&pattern).case_insensitive(true).build()
        .map(|r| r.replace_all(text, "••••••").into_owned())
        .unwrap_or_else(|_| text.replace(code, "••••••"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn masking_covers_normalized_code_variants() {
        for (text, code) in [("验证码 １２３４５６", "123456"), ("验证码 12\u{200b}3456", "123456"), ("验证码 123456", "123 456"), ("验证码 ＡＢＣ－１２３", "ABC-123")] {
            assert_eq!(mask(text, code), "验证码 ••••••", "归一化提取出的验证码必须遮住原文");
        }
    }
}

macro_rules! impls {
    ($($id:ty;)*) => {
        $(
            impl_any_serde!($id, stringify!($id));
            impl_any_json!($id);
            impl_any_binary!($id);
        )*
    }
}

impls! {
    std::string::String;
    std::vec::Vec<u8>;
    u8;u16;u32;u64;u128;
    i8;i16;i32;i64;i128;
}

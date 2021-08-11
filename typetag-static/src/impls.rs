macro_rules! impls{
    ($($id:ty;)*) => {
        $(
            impl_any_serde!($id, stringify!($id));
            //impl_any_json!($id);
            //impl_any_binary!($id);
        )*
    }
}

impls! {
    std::string::String;
    u32;
}

/// mock相关
pub mod mock {
    /// 一个简单的`mock`包装trait
    pub trait SimpleMockable {
        /// 生成一个mock
        fn mock() -> Self;
    }
}

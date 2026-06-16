use std::sync::OnceLock;

use tokio::sync::{RwLock, TryLockError};

/// `QuickFetch`可能导致的错误
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `TryLock`出错
    #[error("failed to lock value: {0}")]
    TryLock(TryLockError),

    /// 变量没有初始化
    #[error("value is not initialize")]
    NotInitialize,
}

type Result<T> = core::result::Result<T, Error>;

/// 用于提供便捷线程安全的全局变量读取的trait
pub trait QuickFetch<T> {
    /// 尝试获取引用
    ///
    /// # Errors
    /// 参见[`Error`]
    fn try_read<U, F>(&self, f: F) -> Result<U>
    where
        F: FnOnce(&T) -> U;

    /// 尝试获取引用。并写入回调函数的返回值
    ///
    /// 当返回值为`Some(T)`时，更新变量；反之，则不更新
    ///
    /// # Errors
    /// 参见[`Error`]
    fn try_write<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(Option<T>) -> Option<T>;

    /// 判断这个变量是否被初始化
    fn is_initialized(&self) -> bool;
}

impl<T> QuickFetch<T> for OnceLock<RwLock<Option<T>>> {
    #[allow(clippy::significant_drop_tightening)]
    fn try_read<U, F>(&self, f: F) -> Result<U>
    where
        F: FnOnce(&T) -> U,
    {
        let binding = self
            .get_or_init(|| RwLock::new(Option::None))
            .try_read()
            .map_err(Error::TryLock)?;
        let x = binding.as_ref().ok_or(Error::NotInitialize)?;
        Ok(f(x))
    }

    #[allow(clippy::significant_drop_tightening)]
    fn try_write<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(Option<T>) -> Option<T>,
    {
        let mut binding = self
            .get_or_init(|| RwLock::new(Option::None))
            .try_write()
            .map_err(Error::TryLock)?;
        *binding = f(binding.take());
        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.get_or_init(|| RwLock::new(Option::None))
            .try_read()
            .ok()
            .is_some_and(|it| it.is_some())
    }
}

/// 一个实用宏，批量检查变量是否初始化，若未初始化，则提前返回Some(&'static str)
#[macro_export]
macro_rules! batch_check_initialized {
    ($($e:expr),*) => {
        {
            $(
                if !$e.is_initialized(){
                    return Some(concat!(stringify!($e)," is not initialized"));
                }
            )*
        }
    };
}

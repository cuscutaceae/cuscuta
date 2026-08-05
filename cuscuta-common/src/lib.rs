//! cuscuta的通用组件
//!
//! # 速览
//! cuscuta-common定义了一些通用的函数、数据模型和一些实用函数，包括各种API，数据库的通用业务操作，定时操作的简易注册和管理，和一些全局变量的线程安全的便捷操作
//!
//! # 局限
//! 很明显，cuscuta并没有做数据层兼容，即cuscuta目前**强绑定于**`Redis`和`PostgreSQL`，这将在未来必要时重构

/// api相关
pub mod api;

/// 通用数据结构相关
pub mod data;

/// 数据库相关
pub mod db;

/// 便捷变量操作相关
pub mod quick_fetch;

/// 定时操作相关
pub mod scheduled_job;

/// 为有参数的枚举类型添加数字转换
#[macro_export]
macro_rules! castable_enum_with_arg {
    (
        $(#[$meta:meta])*
        #repr($repr:ty)
        $vis:vis enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $Variant:ident$(($($v:tt)*))? = $code:expr,
            )*
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $Variant $(($($v)*))?,
            )*
        }
        impl From<$name> for $repr {
            fn from(v: $name) -> Self {
                match v {
                    $($name::$Variant {..} => $code,)*
                }
            }
        }
    };
}

/// 为无参数的枚举类型添加数字转换
#[macro_export]
macro_rules! castable_enum {
    (
        $(#[$meta:meta])*
        #repr($repr:ty)
        $vis:vis enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $Variant:ident = $code:expr,
            )*
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $Variant,
            )*
        }
        impl From<$name> for $repr {
            fn from(v: $name) -> Self {
                match v {
                    $($name::$Variant => $code,)*
                }
            }
        }
        impl From<$repr> for $name {
            fn from(v: $repr) -> Self {
                match v {
                    $($code => $name::$Variant,)*
                    _ => Self::Unknown
                }
            }
        }
    };
}

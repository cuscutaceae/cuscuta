//! > [!CAUTION]
//! >
//! > 测试相关的组件
//! >
//! > 这个文件内存放的mod, trait, fn 皆为 **测试** 相关
//! >
//! > 它们的实现细节相比真实情况做出了可能合理但 **绝对高度的简化**
//! >
//! > 所以它们 **不应** 被用于任何除测试以外的场景

/// mock
pub mod mocks;

/// mock相关
pub mod mock {
    /// 一个简单的`mock`包装trait
    pub trait SimpleMockable {
        /// 生成一个mock
        fn mock() -> Self;
    }
}

/// 测试容器创建相关
pub mod container {
    /// 启动一个redis测试容器并获取[`redis::Client`]
    #[macro_export]
    macro_rules! redis_client_image {
        () => {{
            use redis::Client;
            use testcontainers::{
                GenericImage, ImageExt,
                core::{ContainerPort, WaitFor},
                runners::AsyncRunner,
            };
            let redis_container = GenericImage::new("redis", "7.2.4")
                .with_exposed_port(ContainerPort::Tcp(6379))
                .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
                .start()
                .await
                .expect("failed to boot image");
            ClientWrap(
                Client::open(format!(
                    "redis://{}:{}",
                    redis_container
                        .get_host()
                        .await
                        .expect("failed to get host"),
                    redis_container
                        .get_host_port_ipv4(6379)
                        .await
                        .expect("failed to get host port")
                ))
                .expect("failed to open redis client"),
                redis_container,
            )
        }};
    }
}

/// redis相关
pub mod redis {

    use redis::Client;

    use cuscuta_common::db::{
        job::{JobEssential, enqueue::write_job},
        redis::sub_queue_postfix,
    };
    use testcontainers::{ContainerAsync, Image};

    /// 为防止[`ContainerAsync<T>`]提前回收的包装
    pub struct ClientWrap<T: Image>(pub Client, pub ContainerAsync<T>);

    /// 不检查任何参数，入队模拟查分任务，仅用作测试
    ///
    /// 就像这样：
    /// ```
    /// use cuscuta_common::db::{
    ///     job::{Job, JobEssential, SubQueue}, redis::{job_sub_queue_redis_key, sub_queue_postfix},
    /// };
    /// use cuscuta_test::{
    ///     mock::SimpleMockable,
    ///     mocks::mock::job_state,
    ///     redis::{ClientWrap, fake_enqueue},
    ///     redis_client_image,
    /// };
    /// use redis::{TypedCommands, streams::StreamReadOptions};
    /// #[tokio::main]
    /// async fn main() {
    ///     let ClientWrap(client, _container) = redis_client_image!();
    ///     let hash = "0000000";
    ///     let friend_code = "123456789";
    ///     let job_timestamp = "1784475024";
    ///     let queue_timestamp = "1784475023";
    ///     fake_enqueue(&client, hash, friend_code, job_timestamp, queue_timestamp)
    ///         .expect("failed to enqueue fake jobs");
    ///     let mut connection = client.get_connection().expect("failed to open connection");
    ///     let sub_queue_postfix = sub_queue_postfix(hash, queue_timestamp, 0, 10);
    ///     let sub_queue =
    ///         SubQueue::try_from(sub_queue_postfix.as_str()).expect("failed to parse SubQueue str");
    ///     let reply = connection
    ///         .xread_options(
    ///             &[&job_sub_queue_redis_key(&sub_queue_postfix)],
    ///             &[">"],
    ///             &StreamReadOptions::default().group("default_group", "mock_id"),
    ///         )
    ///         .expect("failed to read from stream")
    ///         .expect("result should be `Some`");
    ///     let stream_id = reply
    ///         .keys
    ///         .first()
    ///         .expect("should have result in stream")
    ///         .ids
    ///         .first()
    ///         .expect("should have result in key");
    ///     let job_deserialized =
    ///         Job::try_from((sub_queue.clone(), stream_id.clone())).expect("failed to parse job");
    ///     assert_eq!(
    ///         job_deserialized.essential,
    ///         JobEssential::new(friend_code.to_string(), job_timestamp.to_string(), 0, 10, 0)
    ///     );
    ///     assert_eq!(
    ///         job_deserialized.sub_queue,
    ///         sub_queue
    ///     );
    /// }
    /// ```
    ///
    /// # Errors
    /// 本函数的错误全部来自[`redis::RedisError`]
    ///
    pub fn fake_enqueue(
        client: &Client,
        hash: &str,
        friend_code: &str,
        job_timestamp: &str,
        queue_timestamp: &str
    ) -> Result<Vec<String>, redis::RedisError> {
        [
            JobEssential::new(friend_code.to_string(), job_timestamp.to_string(), 0, 10, 0),
            JobEssential::new(friend_code.to_string(), job_timestamp.to_string(), 10, 10, 0),
            JobEssential::new(friend_code.to_string(), job_timestamp.to_string(), 20, 10, 0),
        ]
        .map(|it| {
            write_job(
                client,
                &it,
                &sub_queue_postfix(
                    hash,
                    queue_timestamp,
                    it.cursor_start.cast_unsigned() as usize,
                    (it.cursor_start + it.cursor_length).cast_unsigned() as usize,
                ),
                true,
                60,
            )
        })
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
    }
}

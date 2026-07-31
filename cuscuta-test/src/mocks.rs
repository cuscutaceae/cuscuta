/// 生成各种结构体的简易mock
pub mod mock {
    use crate::mock::SimpleMockable;
    use cuscuta_common::{
        api::xxxxxx::FriendInfo,
        data::{Difficulty, Song},
        db::job::{Job, JobEssential, JobFailure, JobFailureResuming, JobFailureType, SubQueue},
    };

    impl SimpleMockable for Job {
        fn mock() -> Self {
            Self {
                job_id: "mock".to_owned(),
                essential: JobEssential::mock(),
                sub_queue: SubQueue::mock(),
                state: job_state::mock_cleaned(),
            }
        }
    }

    impl SimpleMockable for FriendInfo {
        fn mock() -> Self {
            Self {
                name: "nofyso".to_owned(),
                user_id: 123_456,
                rating: 16,
                character: 0,
                is_char_uncapped: false,
                is_char_uncapped_override: false,
            }
        }
    }

    impl SimpleMockable for Vec<Song> {
        fn mock() -> Self {
            vec![
                Song {
                    idx: 0,
                    id: "mock0".to_owned(),
                    difficulties: vec![
                        Difficulty {
                            rating_class: 0,
                            rating: 2,
                        },
                        Difficulty {
                            rating_class: 1,
                            rating: 2,
                        },
                    ],
                },
                Song {
                    idx: 1,
                    id: "mock1".to_owned(),
                    difficulties: vec![
                        Difficulty {
                            rating_class: 0,
                            rating: 2,
                        },
                        Difficulty {
                            rating_class: 1,
                            rating: 2,
                        },
                    ],
                },
                Song {
                    idx: 2,
                    id: "mock2".to_owned(),
                    difficulties: vec![
                        Difficulty {
                            rating_class: 0,
                            rating: 2,
                        },
                        Difficulty {
                            rating_class: 1,
                            rating: 2,
                        },
                    ],
                },
            ]
        }
    }

    /// [`cuscuta_common::db::job::JobState`]的各种mock
    pub mod job_state {
        use cuscuta_common::{
            api::xxxxxx::FriendInfo,
            db::job::{JobFailure, JobState},
        };

        use crate::mock::SimpleMockable;

        /// 生成一个mock的[`JobState`]
        #[must_use]
        pub const fn mock_cleaned() -> JobState {
            JobState::Cleaned
        }

        /// 生成一个mock的[`JobState`]
        #[must_use]
        pub const fn mock_pulled() -> JobState {
            JobState::Pulled {
                start_timestamp: 1_784_475_024,
            }
        }

        /// 生成一个mock的[`JobState`]
        #[must_use]
        pub fn mock_pending() -> JobState {
            JobState::Pending {
                friend_info: FriendInfo::mock(),
                current_length: 0,
                start_timestamp: 1_784_475_024,
            }
        }

        /// 生成一个mock的[`JobState`]
        #[must_use]
        pub fn mock_finished() -> JobState {
            JobState::Finished {
                friend_info: FriendInfo::mock(),
                start_timestamp: 1_784_475_024,
            }
        }

        /// 生成一个mock的[`JobState`]
        #[must_use]
        pub fn mock_failed() -> JobState {
            JobState::Failed {
                friend_info: Some(FriendInfo::mock()),
                start_timestamp: 1_784_475_024,
                failure_info: JobFailure::mock(),
            }
        }
    }

    impl SimpleMockable for JobFailure {
        fn mock() -> Self {
            Self {
                fail_type: JobFailureType::FriendNotFound,
                resume_strategy: JobFailureResuming::Drop,
                timestamp_millis: 1_784_475_024,
            }
        }
    }

    impl SimpleMockable for SubQueue {
        fn mock() -> Self {
            Self {
                name: "mock".to_owned(),
                hash: "mock".to_owned(),
                timestamp: 1_784_475_024,
                segment: 0..5,
            }
        }
    }

    impl SimpleMockable for JobEssential {
        fn mock() -> Self {
            Self {
                friend_code: "123456789".to_owned(),
                timestamp: "1784475024".to_owned(),
                cursor_start: 0,
                cursor_length: 0,
                retry_count: 0,
                job_uid: "abcde".to_owned(),
            }
        }
    }
}

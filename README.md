## cuscuta
[![CI](https://github.com/cuscutaceae/cuscuta/actions/workflows/pr_check.yaml/badge.svg)](https://github.com/cuscutaceae/cuscuta/actions/workflows/pr_check.yaml)
[![Build](https://github.com/cuscutaceae/cuscuta/actions/workflows/build_image.yaml/badge.svg)](https://github.com/cuscutaceae/cuscuta/actions/workflows/build_image.yaml)
[![Helm Chart](https://img.shields.io/badge/helm%20chart-ghcr.io-blue)](https://github.com/cuscutaceae/cuscuta/pkgs/container/charts%2Fcuscuta)
[![cuscuta-entry](https://img.shields.io/badge/cuscuta%20entry-ghcr.io-blue)](https://github.com/cuscutaceae/cuscuta/pkgs/container/cuscuta-entry)
[![cuscuta-worker](https://img.shields.io/badge/cuscuta%20worker-ghcr.io-blue)](https://github.com/cuscutaceae/cuscuta/pkgs/container/cuscuta-worker)
[![cuscuta-chilo](https://img.shields.io/badge/cuscuta%20chilo-ghcr.io-blue)](https://github.com/cuscutaceae/cuscuta/pkgs/container/cuscuta-chilo)
[![License](https://img.shields.io/github/license/cuscutaceae/cuscuta)](LICENSE)

欢迎来到cuscuta的Sekai！ヾ(^▽^*) ……虽然这里乏味而无趣（并且还未完成）

cuscuta是一个分布式爬虫，用于爬取xxxxxx的成绩数据

### 部署

cuscuta提供了Docker compose和Helm两种部署方法，推荐使用Helm部署方法

部署指南参见[部署（简体中文）](Deployment.md) | [Deployment (English)](Deployment.en.md)

### 接入

cuscuta是一个Web服务，使用RESTful API暴露服务，若欲调用现有的cuscuta服务，请参考[OpenAPI文档](docs/openapi.yaml)

### 详情

cuscuta包括一些组件

| 名称           | 用途                                                                                    |
| -------------- | --------------------------------------------------------------------------------------- |
| cuscuta-entry  | 处理入站流量，暴露服务，分发任务，目前理论 **不可扩展（仅单例）**                       |
| cuscuta-worker | 实际处理任务，理论可扩展                                                                |
| cuscuta-common | entry和worker的通用组件                                                                 |
| cuscuta-chilo  | [chilo](https://github.com/cuscutaceae/chilo)的一个WebAPI包装，被worker依赖，理论可扩展 |
| cuscuta-mock   | worker的mock用镜像                                                                      |
| cuscutactl     | cuscuta集群的一个简易命令行管理工具（半数以上使用AI生成）                               |

### 设计与原理

关于cuscuta的设计，参见[cuscuta的草稿 - 4](https://blog.nofyso.cc/2026/05/27/cuscuta-4/)

### 致谢

感谢[@qianmo2233](https://github.com/qianmo2233)对架构设计的支持！

感谢[@Hoyoak](https://www.cnblogs.com/Hoyoak)提供的题解！

Development of _cuscutaceae_ is made possible by contributors like you!

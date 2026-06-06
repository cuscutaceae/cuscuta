## cuscuta

欢迎来到cuscuta的Sekai！ヾ(^▽^*) ……虽然这里乏味而无趣（并且还未完成）

cuscuta是一系列小型软件的集合，组合起来的话……可以用来干一些事情……当然，实验性……

### 组件

cuscuta内含了一些组件

| 名称           | 状态   | 约束                                                   | 用途                                                         |
| -------------- | ------ | ------------------------------------------------------ | ------------------------------------------------------------ |
| cuscuta-entry  | 不稳定 | `#![deny(clippy::pedantic)]`                           | 处理入站流量，暴露服务，分发任务，目前理论 **不可扩展（仅单例）** |
| cuscuta-worker | 不稳定 | `#![deny(clippy::pedantic)]`                           | 实际处理任务，理论可扩展                                     |
| cuscuta-common | 较稳定 | `#![deny(clippy::pedantic)]`, `#![deny(missing_docs)]` | entry和worker的通用组件                                      |
| cuscuta-chilo  | 不稳定 | `#![deny(clippy::pedantic)]`                           | [chilo](https://github.com/cuscutaceae/chilo)的一个WebAPI包装，被worker依赖，理论可扩展 |
| cuscuta-mock   | 不稳定 | 无                                                     | worker的mock用镜像                                           |

### 依赖

cuscuta不能独立工作，它依赖一些其它组件和外部服务工作：

+ PostgreSQL：用来存放worker需要使用的信息
+ Redis：工作队列与缓存
+ 外部服务：就是外部服务……嗯对（在内部测试时可以使用mock代替）

### 部署

非常抱歉，由于cuscuta尚未完工，目前无法部署为helm服务

部分部署教程参见[部署](Deployment.md)

### 计划表

#### cuscuta-worker

- [x] 基础查分服务
- [ ] 错误恢复
  - [ ] 优雅停机时的错误恢复
    - [x] 实现
    - [ ] 测试
  - [ ] 非优雅停机时的错误恢复
    - [x] 实现
    - [ ] 测试
- [ ] 未来功能（可能有）（画饼）
  - [ ] B30专攻快速查询

#### cuscuta-entry

- [ ] 基础端点
  - [x] 任务入列
    - [x] 基本实现
    - [x] 测试
  - [ ] 任务查询
    - [x] 基础查询
    - [x] token检查
      - [ ] 测试
- [ ] 额外端点
  - [ ] chilo状态

#### cuscuta-chilo

- [x] 基础调用
- [ ] chilo热更新 **（重要）**

#### 部署

- [x] docker compose
- [ ] helm

#### cuscutactl（什）

- [ ] 功能
  - [ ] 查询队列状态
  - [ ] 基本数据库CRUD
  - [ ] 查询chilo状态
- [ ] 多连接模式
  - [ ] 数据库直连模式
  - [ ] Kubernetes代理模式

### 设计

关于cuscuta的设计，参见[cuscuta的草稿 - 4](https://blog.nofyso.cc/2026/05/27/cuscuta-4/)

### 致谢

感谢[@qianmo2233](https://github.com/qianmo2233)对架构设计的支持！

感谢[@Hoyoak](https://www.cnblogs.com/Hoyoak)提供的题解！

Development of _cuscutaceae_ is made possible by contributors like you!

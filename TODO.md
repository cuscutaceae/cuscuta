## 计划表

### cuscuta-worker

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

### cuscuta-entry

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

### cuscuta-chilo

- [x] 基础调用
- [ ] chilo热更新 **（重要）**

### 部署

- [x] docker compose
- [x] helm

### cuscutactl

- [ ] 功能
  - [x] 查询队列状态
  - [x] 基本数据库CRUD
  - [ ] 查询chilo状态
- [x] 多连接模式
  - [x] 数据库直连模式
  - [x] Kubernetes代理模式

## 作业
1. 实现 recall
2. 实现 remind

### 1 proto 中定义的请求和响应
```proto
// last visited or watched in X days, given them something to watch
rpc Recall(RecallRequest) returns (RecallResponse);

// last watched in X days, and user still have unfinished contents
rpc Remind(RemindRequest) returns (RemindResponse);
```
1. 对于 recall 根据定义使用最后访问时间作为条件进行查询  `last_visited_at`，并返回推荐的内容
2. 对于 remind 根据定义使用最后观看时间作为条件进行查询  `last_watched_at`，
   并返回用户浏览但未观看的内容（viewed_but_not_started）开始但未完成的视频id（started_but_not_finished）提醒用户

### 2 修改 User 定义
为了能够从数据库中获取数据用户未观看和未完成的内容信息，需要在 `User` 定义中加入 `viewed_but_not_started` 和 `started_but_not_finished` 字段
具体内容如下:

```proto
message User {
    string email = 1;
    string name = 2;
    repeated int64 viewed_but_not_started = 3;
    repeated int64 started_but_not_finished = 4;
}
```
> 用于 Sqlx 的限制此处使用 int64 类型

### 3 sqlx 类型转换
因为数据库中定义的 `viewed_but_not_started` 和 `started_but_not_finished` 字段都是 `int[]`(Vec<int32>) 类型，所以需要对其进行转换成 Vec<i64>

对应源码：https://github.com/kailanyue/06-crm/blob/main/user-stat/src/abi/mod.rs#L25-L73

```rust
pub async fn raw_query(&self, req: RawQueryRequest) -> ServiceResult<ResponseStream> {
    let Ok(rows) = sqlx::query(&req.query).fetch_all(&self.inner.pool).await else {
        return Err(Status::internal(format!(
            "Failed to fetch data with query: {}",
            req.query
        )));
    };

    let ret: Vec<User> = rows
        .into_iter()
        .map(|row| {
            let email: String = row
                .try_get("email")
                .map_err(|e| Status::internal(format!("Failed to get email: {}", e)))?;

            let name: String = row
                .try_get("name")
                .map_err(|e| Status::internal(format!("Failed to get name: {}", e)))?;

            let viewed_but_not_started: Vec<i32> =
                row.try_get("viewed_but_not_started").map_err(|e| {
                    Status::internal(format!("Failed to get viewed_but_not_started: {}", e))
                })?;

            let started_but_not_finished: Vec<i32> =
                row.try_get("started_but_not_finished").map_err(|e| {
                    Status::internal(format!("Failed to get started_but_not_finished: {}", e))
                })?;

            Ok(User {
                email,
                name,
                viewed_but_not_started: viewed_but_not_started
                    .into_iter()
                    .map(|i| i as i64)
                    .collect(),
                started_but_not_finished: started_but_not_finished
                    .into_iter()
                    .map(|i| i as i64)
                    .collect(),
            })
        })
        .collect::<Result<Vec<User>, Status>>()?;

    Ok(Response::new(Box::pin(futures::stream::iter(
        ret.into_iter().map(Ok),
    ))))
}
```

### 4 recall 实现
使用 `last_visited_at` 作为查询限制条件，具体实现细节同 `welcome`，

在 client 中调用
源码：https://github.com/kailanyue/06-crm/blob/main/crm/src/client.rs
```rust
// recall request
let recall_request = RecallRequestBuilder::default()
    .id(Uuid::new_v4().to_string())
    .last_visit_interval(20u32)
    .content_ids([1u32, 2, 3])
    .build()?;

let response_recall = client
    .recall(Request::new(recall_request))
    .await?
    .into_inner();
println!("Recall Response: {:?}", response_recall);

```

### 5 remind 实现
使用 `last_watched_at` 作为查询限制条件，整体实现同 `welcome`，细节上略有不同

1. 定义 UnfinishedContents 记录用户未观看和未完成的内容信息的内容，定义如下

```proto
message UnfinishedContents{
    string description = 1;
    repeated int64 viewed_but_not_started = 2;
    repeated int64 started_but_not_finished = 3;
}
```

2. 为 `UnfinishedContents` 实现 `fmt::Display`
源码：https://github.com/kailanyue/06-crm/blob/main/crm-metadata/src/abi/mod.rs#L102-L110
```rust
impl fmt::Display for UnfinishedContents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "description: {}, viewed_but_not_started: {:?}, started_but_not_finished: {:?}",
            self.description, self.viewed_but_not_started, self.started_but_not_finished
        )
    }
}
```

3. 创建新的 new_remind 方法，能够接受 `viewed_but_not_started` 和 `started_but_not_finished 作为参数`
源码：https://github.com/kailanyue/06-crm/blob/main/crm-send/src/abi/mod.rs#L102-L119
```rust
impl SendRequest {
    pub fn new_remind(
        subject: String,
        sender: String,
        recipients: &[String],
        viewed_but_not_started: Vec<i64>,
        started_but_not_finished: Vec<i64>,
    ) -> Self {
        let contents = UnfinishedContents::new(viewed_but_not_started, started_but_not_finished);

        let msg = Msg::Email(EmailMessage {
            message_id: Uuid::new_v4().to_string(),
            subject,
            sender,
            recipients: recipients.to_vec(),
            body: contents.to_string(),
        });
        SendRequest { msg: Some(msg) }
    }
}
```

4. 在CrmService中实现 `recall` 方法
源码：https://github.com/kailanyue/06-crm/blob/main/crm/src/abi/mod.rs
```rust
pub async fn remind(&self, request: RemindRequest) -> Result<Response<RemindResponse>, Status> {
    let mut res_user_stats = self
        .query_user_stats(
            "last_watched_at",
            Duration::days(request.last_visit_interval as i64),
            Duration::days(1),
        )
        .await?;
    let (tx, rx) = mpsc::channel(CHANNEL_SIZE);

    let sender = self.config.server.sender_email.clone();

    tokio::spawn(async move {
        while let Some(Ok(user)) = res_user_stats.next().await {
            let sender = sender.clone();
            let tx = tx.clone();

            println!("Remind: {:?}", user.name);
            let req = SendRequest::new_remind(
                "Remind".to_string(),
                sender,
                &[user.email],
                user.viewed_but_not_started,
                user.started_but_not_finished,
            );

            if let Err(e) = tx.send(req).await {
                warn!("Failed to send message: {:?}", e);
            }
        }
    });

    let reqs = ReceiverStream::new(rx);

    self.notification.clone().send(reqs).await?;
    Ok(Response::new(RemindResponse { id: request.id }))
}

```

5. 在 client 中调用
```rust
// remind request
let remind_request = RemindRequestBuilder::default()
    .id(Uuid::new_v4().to_string())
    .last_visit_interval(20u32)
    .build()?;

let response_remind = client
    .remind(Request::new(remind_request))
    .await?
    .into_inner();
println!("Remind Response: {:?}", response_remind);
```


An implementation of `futures::Future` that is sorta like a Java `CompletableFuture`

```
let mut future = CompletableFuture::<Result<u32, String>>::new();

let local = future.clone();
std::thread::spawn(move || {
    let val = local.wait_timeout(Duration::from_secs(1));
    println!("{:?}", val);
});

future.complete(Ok(42));
```

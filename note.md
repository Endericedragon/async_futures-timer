# async_futures-timer改造计划

## 代码阅读

由于async-os的设计理念和`futures-timer`原本支持的系统很不一样，因此必须完全搞懂后者才能移植到前者。先来看看它的示例用法：

```rust
use std::time::Duration;
use futures_timer::Delay;

#[async_std::main]
async fn main() {
    let now = Delay::new(Duration::from_secs(3)).await;
    println!("waited for 3 secs");
}
```

这东西的作用倒是简单，就是阻塞一个协程一段时间（这段时间的长短由`Duration`类型的参数决定）。我们来看看这个效果该如何在async-os中实现呢？

查阅`apps/helloworld`可知，`async_std::thread::sleep()`和它想做的事情一模一样。这样，不如直接用上述函数实现一个新的`Delay`结构体不就成了？

## 问题汇总

> **问题**：`modules/async_futures-timer/src/native/heap_timer.rs`中报告，`Instant`不能使用`==, <`等算数符号。

为`Instant`继承`PartialEq, Eq, PartialOrd, Ord` trait即可。

> **问题**：`modules/async_futures-timer/src/native/global.rs`报告，无法使用`"some text".to_owned()`

在代码开头加上一句`use alloc::borrow::ToOwned;`即可。

> `modules/async_futures-timer/src/native/global.rs`报告，`let thread = thread::Builder::new().name("futures-timer".to_owned()).spawn(move || run(timer, done2))?;`中，闭包不是个`Future`

目前使用`async`块包裹闭包，消除了报错，但不确定是否有改变原本语义：

```rust
let thread = thread::Builder::new()
    .name("futures-timer".to_owned())
    .spawn(async {
        move || run(timer, done2);
    })?;
```

> **重大问题**：在所有用到了`std::sync::Mutex`的代码文件中均报告，no method named `unwrap` found for struct `MutexGuardFuture` in the current scope

对比原版`std`和`async_std`库可知，二者定义的`Mutex`用法有所不同：

```rust
use std::sync::Mutex;

let mutex = Mutex::new(5);

let mut lock: Result<MutextGuard<'_, i32>, ...> = mutex.lock();
// 原版`std`库中，`Mutex`是这样使用的：
let mut data = lock.unwrap();
// 而在`async_std`库中，`Mutex`是这样使用的（参考`apps/helloworld`示例）：
let mut data = lock.await;
```

也就是说，原版`Mutex`是为同步程序设计，因此在等待锁时使用的是`unwrap`；而async-os是异步设计，因此在等待锁时用的是`await`。

为此，ZFL前辈提出建议：使用StarryOS的同步环境下的Mutex库，而非`async_std`库中的Mutex。笔者已经[fork了一份](https://github.com/Endericedragon/async_axsync_with_sync)。不过ZFL前辈也指出这个库不能直接在async-os中使用，需要进行改造。
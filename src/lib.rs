use futures::stream::FuturesUnordered;
use futures::Future;
use futures::StreamExt;
use std::pin::Pin;
use tokio::sync::mpsc;
use std::fmt::Debug;

type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

pub struct TaskPool<Out> {
    capacity: usize,
    input: mpsc::Receiver<BoxFuture<Out>>,
    output: mpsc::Sender<Out>,
    queue: Vec<BoxFuture<Out>>,
    pool: FuturesUnordered<BoxFuture<Out>>,
}

impl<Out: 'static + Debug> TaskPool<Out> {
    fn new() -> (Self, mpsc::Sender<BoxFuture<Out>>, mpsc::Receiver<Out>) {
        let (in_tx, in_rx) = mpsc::channel(1);
        let (out_tx, out_rx) = mpsc::channel(1);

        (
            Self {
                capacity: 2,
                input: in_rx,
                output: out_tx,
                queue: Vec::new(),
                pool: FuturesUnordered::new(),
            },
            in_tx,
            out_rx,
        )
    }

    async fn start(&mut self) {
        loop {
            tokio::select! {
                input = self.input.recv() => {
                    if let Some(input) = input {
                        if self.pool.len() < self.capacity {
                            self.pool.push(input);
                        } else {
                            self.queue.push(input);
                        }
                    }
                }
                maybe_result = self.pool.next() => {
                    if let Some(result) = maybe_result {
                        println!("{:?}", result);
                        self.output.send(result).await.unwrap();
                        if let Some(input) = self.queue.pop() {
                            self.pool.push(input);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {

        async fn transform(duration: u64) -> u64 {
            tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;
            duration
        }

        let (mut pool, input, mut output) = TaskPool::new();

        input.send(Box::pin(transform(1))).await.unwrap();

        loop {
            tokio::select! {
                out = output.recv() => {
                    println!("{:?}", out);
                    if let Some(duration) = out {
                        input.send(Box::pin(transform(duration + 1))).await.unwrap();
                    }
                }
                _ = pool.start() => {
                }
            }
        }

    }
}

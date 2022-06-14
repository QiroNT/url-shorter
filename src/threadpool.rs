use std::{
  collections::VecDeque,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
  },
  thread,
  time::Duration,
};

use parking_lot::{Condvar, Mutex};

pub struct ThreadPool {
  data: Arc<SharedData>,
  min: usize,
  max: usize,
  timeout: Duration,
}

struct SharedData {
  queue: Mutex<VecDeque<Box<dyn FnOnce() + Send + 'static>>>,
  condvar: Condvar,
  active_workers: AtomicUsize,
  idle_workers: AtomicUsize,
}

struct Registration<'a>(&'a AtomicUsize);

impl<'a> Registration<'a> {
  fn new(counter: &'a AtomicUsize) -> Self {
    counter.fetch_add(1, Ordering::Release);
    Registration(counter)
  }
}

impl<'a> Drop for Registration<'a> {
  fn drop(&mut self) {
    self.0.fetch_sub(1, Ordering::Release);
  }
}

impl ThreadPool {
  pub fn new(min: usize, max: usize, timeout: Duration) -> Self {
    let pool = ThreadPool {
      data: Arc::new(SharedData {
        queue: Mutex::new(VecDeque::new()),
        condvar: Condvar::new(),
        active_workers: AtomicUsize::new(0),
        idle_workers: AtomicUsize::new(0),
      }),
      min,
      max,
      timeout,
    };

    for _ in 0..min {
      pool.spawn_worker(|| {});
    }

    pool
  }

  fn spawn_worker<F>(&self, init_job: F)
  where
    F: FnOnce() + Send + 'static,
  {
    let data = self.data.clone();
    let min = self.min;
    let timeout = self.timeout;

    thread::spawn(move || {
      let _active_guard = Registration::new(&data.active_workers);

      init_job();

      loop {
        let job;

        let mut queue = data.queue.lock();
        loop {
          if let Some(poped_task) = queue.pop_front() {
            job = poped_task;
            break;
          }

          let _idle_guard = Registration::new(&data.idle_workers);

          if data.active_workers.load(Ordering::Acquire) <= min {
            data.condvar.wait(&mut queue);
          } else {
            let wait_res = data.condvar.wait_for(&mut queue, timeout);

            if wait_res.timed_out() && queue.is_empty() {
              return;
            }
          }
        }
        drop(queue);

        job();
      }
    });
  }

  pub fn execute<F>(&self, job: F)
  where
    F: FnOnce() + Send + 'static,
  {
    let mut queue = self.data.queue.lock();

    if self.data.idle_workers.load(Ordering::Acquire) == 0
      && self.data.active_workers.load(Ordering::Acquire) < self.max
    {
      self.spawn_worker(job);
    } else {
      queue.push_back(Box::new(job));
      self.data.condvar.notify_one();
    }
  }
}

impl Drop for ThreadPool {
  fn drop(&mut self) {
    self
      .data
      .active_workers
      .store(usize::MAX / 2, Ordering::Release);
    self.data.condvar.notify_all();
  }
}

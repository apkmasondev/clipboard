type Subscriber = { resolve: (value: string) => void; reject: (error: unknown) => void };
type Job = { id: string; subscribers: Set<Subscriber>; started: boolean; generation: number };

/** Bounded in-memory LRU and shared requests. Cancelled, offscreen jobs never start. */
export class PreviewQueue {
  private cache = new Map<string, string>();
  private bytes = 0;
  private jobs = new Map<string, Job>();
  private queue: Job[] = [];
  private running = 0;
  private generation = 0;

  constructor(
    private load: (id: string) => Promise<string>,
    private maxBytes = 4 * 1024 * 1024,
    private maxEntries = 32,
    private concurrency = 2,
  ) {}

  clear() {
    this.cache.clear();
    this.bytes = 0;
    this.generation++;
  }

  request(id: string, signal: AbortSignal): Promise<string> {
    if (signal.aborted) return Promise.reject(new DOMException('Anulowano', 'AbortError'));
    const cached = this.cache.get(id);
    if (cached !== undefined) {
      this.cache.delete(id);
      this.cache.set(id, cached);
      return Promise.resolve(cached);
    }
    let job = this.jobs.get(id);
    if (!job) {
      job = { id, subscribers: new Set(), started: false, generation: this.generation };
      this.jobs.set(id, job);
      this.queue.push(job);
    }
    const current = job;
    const promise = new Promise<string>((resolve, reject) => {
      const subscriber: Subscriber = {
        resolve: (value) => {
          signal.removeEventListener('abort', abort);
          resolve(value);
        },
        reject: (error) => {
          signal.removeEventListener('abort', abort);
          reject(error);
        },
      };
      const abort = () => {
        current.subscribers.delete(subscriber);
        subscriber.reject(new DOMException('Anulowano', 'AbortError'));
        if (!current.started && current.subscribers.size === 0) {
          this.queue = this.queue.filter((candidate) => candidate !== current);
          this.jobs.delete(id);
        }
      };
      current.subscribers.add(subscriber);
      signal.addEventListener('abort', abort, { once: true });
    });
    this.pump();
    return promise;
  }

  private pump() {
    while (this.running < this.concurrency && this.queue.length) {
      const job = this.queue.shift()!;
      if (!job.subscribers.size) continue;
      job.started = true;
      this.running++;
      Promise.resolve()
        .then(() => this.load(job.id))
        .then((value) => {
          if (job.subscribers.size && job.generation === this.generation)
            this.remember(job.id, value);
          this.jobs.delete(job.id);
          for (const subscriber of job.subscribers) subscriber.resolve(value);
        })
        .catch((error: unknown) => {
          this.jobs.delete(job.id);
          for (const subscriber of job.subscribers) subscriber.reject(error);
        })
        .finally(() => {
          job.subscribers.clear();
          if (this.jobs.get(job.id) === job) this.jobs.delete(job.id);
          this.running--;
          this.pump();
        });
    }
  }

  private remember(id: string, value: string) {
    // Count UTF-16 conservatively, even on engines that store ASCII more compactly.
    const bytes = value.length * 2;
    if (bytes > this.maxBytes) return;
    this.cache.set(id, value);
    this.bytes += bytes;
    while (this.bytes > this.maxBytes || this.cache.size > this.maxEntries) {
      const oldest = this.cache.keys().next().value!;
      this.bytes -= this.cache.get(oldest)!.length * 2;
      this.cache.delete(oldest);
    }
  }
}

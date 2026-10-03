import { describe, it, expect, vi } from 'vitest';
import { PreviewQueue } from './previewQueue';

const signal = () => new AbortController().signal;
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve();
};
describe('image preview queue', () => {
  it('shares an in-flight image between list and selected preview', async () => {
    const load = vi.fn(async () => 'png');
    const queue = new PreviewQueue(load);
    expect(await Promise.all([queue.request('a', signal()), queue.request('a', signal())])).toEqual(
      ['png', 'png'],
    );
    expect(load).toHaveBeenCalledTimes(1);
    expect(await queue.request('a', signal())).toBe('png');
    expect(load).toHaveBeenCalledTimes(1);
  });
  it('bounds parallel requests and cancels offscreen queued work', async () => {
    const finishes: Array<(value: string) => void> = [];
    const load = vi.fn(() => new Promise<string>((resolve) => finishes.push(resolve)));
    const queue = new PreviewQueue(load);
    const first = queue.request('a', signal());
    const second = queue.request('b', signal());
    const controller = new AbortController();
    const cancelled = queue.request('c', controller.signal).catch((error: Error) => error.name);
    controller.abort();
    const fourth = queue.request('d', signal());
    await flush();
    expect(load.mock.calls).toHaveLength(2);
    expect(await cancelled).toBe('AbortError');
    finishes[0]('a');
    finishes[1]('b');
    await Promise.all([first, second]);
    await flush();
    expect(load).toHaveBeenCalledTimes(3);
    expect(load).toHaveBeenLastCalledWith('d');
    finishes[2]('d');
    await fourth;
  });
  it('aborting one subscriber does not cancel another or cache abandoned work', async () => {
    let finish!: (value: string) => void;
    const load = vi.fn(
      () =>
        new Promise<string>((resolve) => {
          finish = resolve;
        }),
    );
    const queue = new PreviewQueue(load);
    const controller = new AbortController();
    const cancelled = queue.request('a', controller.signal).catch((error: Error) => error.name);
    const visible = queue.request('a', signal());
    controller.abort();
    await flush();
    finish('a');
    expect(await cancelled).toBe('AbortError');
    expect(await visible).toBe('a');
    const controller2 = new AbortController();
    const abandoned = queue.request('b', controller2.signal).catch(() => undefined);
    await flush();
    controller2.abort();
    finish('b');
    await abandoned;
    await flush();
    const retry = queue.request('b', signal());
    await flush();
    finish('fresh');
    expect(await retry).toBe('fresh');
    expect(load).toHaveBeenCalledTimes(3);
  });
  it('evicts by memory and LRU count and does not cache over-budget previews', async () => {
    const load = vi.fn(async (id: string) => id.repeat(3));
    const queue = new PreviewQueue(load, 12, 2);
    await queue.request('a', signal());
    await queue.request('b', signal());
    await queue.request('a', signal());
    await queue.request('c', signal());
    await queue.request('a', signal());
    expect(load).toHaveBeenCalledTimes(3);
    await queue.request('b', signal());
    expect(load).toHaveBeenCalledTimes(4);
    await queue.request('large', signal());
    await queue.request('large', signal());
    expect(load).toHaveBeenCalledTimes(6);
    const byCount = new PreviewQueue(load, 100, 1);
    await byCount.request('a', signal());
    await byCount.request('b', signal());
    await byCount.request('a', signal());
    expect(load).toHaveBeenCalledTimes(9);
  });
  it('clearing cache during a request prevents stale repopulation', async () => {
    let finish!: (value: string) => void;
    const load = vi.fn(
      () =>
        new Promise<string>((resolve) => {
          finish = resolve;
        }),
    );
    const queue = new PreviewQueue(load);
    const first = queue.request('a', signal());
    await flush();
    queue.clear();
    finish('old');
    await first;
    await flush();
    const second = queue.request('a', signal());
    await flush();
    finish('new');
    expect(await second).toBe('new');
    expect(load).toHaveBeenCalledTimes(2);
  });
  it('recovers immediately after corrupt or deleted images without hanging the queue', async () => {
    const load = vi.fn().mockRejectedValueOnce(new Error('deleted')).mockResolvedValue('valid');
    const queue = new PreviewQueue(load);
    await expect(queue.request('a', signal())).rejects.toThrow('deleted');
    expect(await queue.request('a', signal())).toBe('valid');
    const controller = new AbortController();
    controller.abort();
    await expect(queue.request('unused', controller.signal)).rejects.toMatchObject({
      name: 'AbortError',
    });
    expect(load).toHaveBeenCalledTimes(2);
  });
});

import { describe, it, expect } from 'vitest';
import { transform } from './transforms';
describe('local transforms', () => {
  it('round trips Unicode including emoji', () => {
    const s = 'Zażółć gęślą jaźń 🧩\n你好';
    expect(transform(transform(s, 'base64Encode'), 'base64Decode')).toBe(s);
    expect(transform(transform(s, 'urlEncode'), 'urlDecode')).toBe(s);
  });
  it('supports large Unicode without call-stack overflow', () => {
    const s = 'ą'.repeat(500000);
    expect(transform(transform(s, 'base64Encode'), 'base64Decode')).toBe(s);
  });
  it('does not silently accept invalid JSON or encoding', () => {
    expect(() => transform('{oops}', 'pretty')).toThrow();
    expect(() => transform('%ZZ', 'urlDecode')).toThrow();
    expect(() => transform('/w==', 'base64Decode')).toThrow();
  });
  it('preserves JSON values', () => {
    const s = '{"x":[null,1,true,"a"]}';
    expect(transform(transform(s, 'pretty'), 'minify')).toBe(s);
  });
  it('preserves large JSON numbers and quoted punctuation', () => {
    const s = '{"id":9007199254740993,"x":1e400,"text":"{a, b: \\\"x\\\"}","empty":[]}';
    expect(transform(transform(s, 'pretty'), 'minify')).toBe(s);
  });
  it('preserves line breaks and strips duplicate lines', () => {
    expect(transform('a  b\n c\t d', 'spaces')).toBe('a b\n c d');
    expect(transform('a\r\nb\r\na', 'unique')).toBe('a\nb');
  });
  it('handles Polish case', () => {
    expect(transform('ŻÓŁTY ŚWIAT', 'title')).toBe('Żółty Świat');
  });
});

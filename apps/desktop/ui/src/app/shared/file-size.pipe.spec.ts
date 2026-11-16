import { formatFileSize } from './file-size.pipe';

describe('formatFileSize', () => {
  it('formats bytes with the closest unit', () => {
    expect(formatFileSize(0)).toBe('0 B');
    expect(formatFileSize(1023)).toBe('1023 B');
    expect(formatFileSize(1536)).toBe('1.5 KB');
    expect(formatFileSize(5 * 1024 ** 3)).toBe('5.0 GB');
  });
});

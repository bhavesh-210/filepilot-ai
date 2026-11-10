"""Private bounded PDF parser worker receiving bytes rather than file paths"""
import io
import json
import sys


def extract(payload, pages, chars):
    import resource
    resource.setrlimit(resource.RLIMIT_CPU, (4, 4))
    memory_limited = sys.platform.startswith('linux')
    if memory_limited:
        resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024,) * 2)
    try:
        from pypdf import PdfReader
    except ImportError:
        return dict(status='skipped', reason='dependency_missing')
    try:
        reader = PdfReader(io.BytesIO(payload), strict=True)
        if reader.is_encrypted:
            return dict(status='skipped', reason='encrypted_pdf')
        total = len(reader.pages)
        text, inspected, truncated = '', 0, total > pages
        for page in list(reader.pages[:pages]):
            stream = page.get_contents()
            if stream is not None and len(stream.get_data()) > 1024 * 1024:
                return dict(status='skipped', reason='decoded_stream_limit')
            value = page.extract_text() or ''
            remaining = chars - len(text)
            text += value[:remaining]
            inspected += 1
            if len(value) > remaining or (len(text) == chars and inspected < min(total, pages)):
                truncated = True
                break
        return dict(status='ok' if text.strip() else 'no_text',
                    reason=None if text.strip() else 'no_extractable_text', text=text,
                    truncated=truncated, pages_inspected=inspected,
                    memory_limit_enforced=memory_limited)
    except Exception:
        return dict(status='skipped', reason='invalid_or_resource_limited_pdf')


if __name__ == '__main__':
    try:
        pages, chars = map(int, sys.argv[1:])
        if not 1 <= pages <= 20 or not 1 <= chars <= 16000:
            raise ValueError()
        payload = sys.stdin.buffer.read(2097153)
        result = extract(payload, pages, chars) if len(payload) <= 2097152 else dict(status='skipped', reason='byte_limit')
    except Exception:
        result = dict(status='skipped', reason='worker_failure')
    print(json.dumps(result, ensure_ascii=True))

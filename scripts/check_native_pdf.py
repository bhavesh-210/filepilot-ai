"""Test the isolated native parser using generated synthetic PDF bytes only"""
import base64
import json
from pathlib import Path
import subprocess
import unittest
HELPER=Path(__file__).resolve().parents[1]/'desktop/src-tauri/parser/filepilot-pdf'

def document(text='Synthetic lecture notes',pages=1):
    objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'',b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>']
    kids=[]
    for _ in range(pages):
        index=len(objects)+1;kids.append(f'{index} 0 R')
        objects.append(f'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents {index+1} 0 R >>'.encode())
        stream=('BT /F1 12 Tf 72 720 Td ('+text+') Tj ET').encode()
        objects.append(b'<< /Length '+str(len(stream)).encode()+b' >>\nstream\n'+stream+b'\nendstream')
    objects[1]=('<< /Type /Pages /Kids ['+' '.join(kids)+f'] /Count {pages} >>').encode()
    result=b'%PDF-1.4\n';offsets=[0]
    for i,value in enumerate(objects,1):
        offsets.append(len(result));result+=f'{i} 0 obj\n'.encode()+value+b'\nendobj\n'
    xref=len(result);result+=f'xref\n0 {len(objects)+1}\n0000000000 65535 f \n'.encode()
    result+=b''.join(f'{offset:010} 00000 n \n'.encode() for offset in offsets[1:])
    result+=f'trailer\n<< /Size {len(objects)+1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n'.encode()
    return result

class PdfWorkerTests(unittest.TestCase):
    def inspect(self,payload):
        process=subprocess.run([str(HELPER)],input=payload,capture_output=True,timeout=6)
        self.assertEqual(process.returncode,0,process.stderr.decode(errors='replace'))
        return json.loads(process.stdout)
    def test_synthetic_text(self):
        value=self.inspect(document());self.assertEqual(value['status'],'ok')
        self.assertIn('Synthetic lecture notes',value['text']);self.assertFalse(value['ocr'])
    def test_empty_and_invalid(self):
        self.assertEqual(self.inspect(document(''))['status'],'no_text')
        self.assertEqual(self.inspect(b'%PDF-1.4 broken')['status'],'skipped')
        self.assertEqual(self.inspect(b'not a PDF')['status'],'skipped')
    def test_page_and_text_limits(self):
        value=self.inspect(document('x'*3000,6));self.assertTrue(value['truncated'])
        self.assertLessEqual(len(value['text']),2000);self.assertLessEqual(value['pages_inspected'],5)
    def test_input_limit(self):
        self.assertEqual(self.inspect(b'%PDF-'+b'x'*2097152)['status'],'skipped')
    def test_encrypted(self):
        value=self.inspect(base64.b64decode('JVBERi0xLjMKJeLjz9MKMSAwIG9iago8PAovUHJvZHVjZXIgPGQ5ZjMxNzJlYWY+Cj4+CmVuZG9iagoyIDAgb2JqCjw8Ci9UeXBlIC9QYWdlcwovQ291bnQgMQovS2lkcyBbIDQgMCBSIF0KPj4KZW5kb2JqCjMgMCBvYmoKPDwKL1R5cGUgL0NhdGFsb2cKL1BhZ2VzIDIgMCBSCj4+CmVuZG9iago0IDAgb2JqCjw8Ci9UeXBlIC9QYWdlCi9SZXNvdXJjZXMgPDwKPj4KL01lZGlhQm94IFsgMC4wIDAuMCAyMCAyMCBdCi9QYXJlbnQgMiAwIFIKPj4KZW5kb2JqCjUgMCBvYmoKPDwKL1YgMQovUiAyCi9MZW5ndGggNDAKL1AgNDI5NDk2NzI5MgovRmlsdGVyIC9TdGFuZGFyZAovTyA8ZDY5YWFlNDQwYmY3Y2NjN2YzNzQyZmY5YWFkMzJhNDQ2ZTFjYzkzNzk5MTRkZWFhNWQ0NTQ5NTFiM2EyODVlYT4KL1UgPGMwNTRlZWE2MGEwOTJkOTc5NzdjMTA5Zjk3ZjYxOWU5NDQ4ZWM1NTU0N2RlODU3YjIyZDI4MzgzYTgyMWQ2NWQ+Cj4+CmVuZG9iagp4cmVmCjAgNgowMDAwMDAwMDAwIDY1NTM1IGYgCjAwMDAwMDAwMTUgMDAwMDAgbiAKMDAwMDAwMDA1OSAwMDAwMCBuIAowMDAwMDAwMTE4IDAwMDAwIG4gCjAwMDAwMDAxNjcgMDAwMDAgbiAKMDAwMDAwMDI1OSAwMDAwMCBuIAp0cmFpbGVyCjw8Ci9TaXplIDYKL1Jvb3QgMyAwIFIKL0luZm8gMSAwIFIKL0lEIFsgPDM1MzM2MTY2NjEzMTM3MzczNzMxNjI2MTY1MzMzNjY1MzYzMzMzNjQzMDYzMzYzMzM4NjMzNDM4NjYzOTM1MzY+IDwzNTMzNjE2NjYxMzEzNzM3MzczMTYyNjE2NTMzMzY2NTM2MzMzMzY0MzA2MzM2MzMzODYzMzQzODY2MzkzNTM2PiBdCi9FbmNyeXB0IDUgMCBSCj4+CnN0YXJ0eHJlZgo0NzMKJSVFT0YK'))
        self.assertEqual(value['status'],'skipped');self.assertEqual(value['reason'],'Encrypted PDF')

if __name__=='__main__':unittest.main(verbosity=2)

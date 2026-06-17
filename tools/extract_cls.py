import zipfile
import os

z = zipfile.ZipFile(r'd:\Code\Rust\LynxOCR\docs\ocr-models\ppocr-v4.zip')
z.extract('cls.onnx', r'd:\Code\Rust\LynxOCR\tools\ppocr-v6')
dst = r'd:\Code\Rust\LynxOCR\tools\ppocr-v6\cls.onnx'
print(f'Extracted cls.onnx: {os.path.getsize(dst) / (1024*1024):.1f} MB')
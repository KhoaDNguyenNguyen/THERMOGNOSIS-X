from pathlib import Path
from thermognosis.dataset.json_parser import stream_samples

for s,p in stream_samples(Path("dataset/raw")):
    print(s.sample_id,p.x,p.y)
    break

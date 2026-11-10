"""Deterministic local lexical ranking without model downloads or file access"""
import math
import re
from collections import Counter
from pathlib import PurePosixPath
from .scanner import CATEGORIES


def relative_path(value):
    if not isinstance(value, str) or not value or len(value) > 4096:
        raise ValueError('Invalid relative path')
    if value.startswith('/') or '\\' in value or any(ord(c) < 32 for c in value):
        raise ValueError('Invalid relative path')
    if any(p in {'', '.', '..'} for p in value.split('/')):
        raise ValueError('Invalid relative path')
    return value


def tokens(value):
    return Counter(re.findall(r'[^\W_]+', value.casefold(), re.UNICODE))


def rank_destinations(filename, category, folders, *, text='', preferred=None, limit=3):
    relative_path(filename)
    if category not in CATEGORIES or not isinstance(text, str) or len(text) > 16000:
        raise ValueError('Invalid ranking input')
    if type(limit) is not int or not 1 <= limit <= 10 or not isinstance(folders, list) or len(folders) > 1000:
        raise ValueError('Invalid ranking limits')
    query = tokens(PurePosixPath(filename).stem + ' ' + text)
    candidates, seen = [], set()
    for folder in folders:
        if not isinstance(folder, dict) or set(folder) != {'path', 'category'}:
            raise ValueError('Invalid folder representation')
        path = relative_path(folder['path'])
        if path in seen or folder['category'] not in CATEGORIES:
            raise ValueError('Invalid or duplicate folder')
        seen.add(path)
        if preferred is not None:
            relative_path(preferred)
        representation = tokens(path.replace('/', ' '))
        denominator = math.sqrt(sum(v*v for v in query.values()) * sum(v*v for v in representation.values()))
        similarity = sum(v * representation[k] for k, v in query.items()) / denominator if denominator else 0
        category_match = category != 'Unknown' and category == folder['category']
        correction = preferred == path
        score = min(1.0, .7 * similarity + .3 * category_match)
        if score > 0 or correction:
            candidates.append(dict(path=path, score=round(score, 6), preference_match=correction,
                                   reasons=(["explicit_local_correction"] if correction else []) +
                                   (["category_match"] if category_match else []) +
                                   (["lexical_overlap"] if similarity else [])))
    candidates.sort(key=lambda x: (-int(x['preference_match']), -x['score'], x['path']))
    return candidates[:limit]


def ranking_metrics(examples):
    if not isinstance(examples, list) or not examples:
        raise ValueError('Provide evaluation examples')
    top1 = recall3 = reciprocal = 0
    for example in examples:
        expected = relative_path(example['expected'])
        ranking = example['ranked']
        if not isinstance(ranking, list) or len(set(ranking)) != len(ranking):
            raise ValueError('Invalid ranking')
        for path in ranking:
            relative_path(path)
        if expected in ranking:
            position = ranking.index(expected) + 1
            top1 += position == 1
            recall3 += position <= 3
            reciprocal += 1 / position
    count = len(examples)
    return dict(examples=count, destination_accuracy=top1/count, recall_at_3=recall3/count,
                mean_reciprocal_rank=reciprocal/count, independent_benchmark=False)

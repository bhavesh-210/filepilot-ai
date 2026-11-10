"""Create reviewable destination proposals without granting execution permission"""
import argparse
import hashlib
import json
from pathlib import PurePosixPath
import sys
import unicodedata
from .content import _excluded
from .policy import PolicySettings, decide
from .ranking import rank_destinations, relative_path
from .report import validate_report


def collision_key(path):
    return unicodedata.normalize('NFC', path).casefold()


def build_plan(report, folders, *, settings=None, preferences=None, scope='default'):
    validate_report(report)
    relative_path(scope)
    settings = settings or PolicySettings()
    if not isinstance(settings, PolicySettings):
        raise ValueError('Invalid policy settings')
    if not isinstance(folders, list):
        raise ValueError('Provide explicit candidate folders')
    entries = {entry['path']: entry for entry in report['entries']}
    occupied = {collision_key(path) for path in entries}
    # Validate all candidate folders before producing any proposals
    rank_destinations('validation.txt', 'Unknown', folders)
    for folder in folders:
        path = relative_path(folder['path'])
        entry = entries.get(path)
        if any(_excluded(part) for part in path.split('/')):
            raise ValueError('Excluded destination')
        if not entry or entry['kind'] != 'directory' or entry['status'] != 'inventoried' or entry['project_root'] is not None:
            raise ValueError('Destinations must be inventoried folders outside projects')
    proposals, untouched = [], []
    incomplete = report['truncated'] or bool(report['errors'])
    for path, entry in entries.items():
        relative_path(path)
        classification = entry['classification']
        if entry['kind'] != 'file' or entry['status'] != 'inventoried' or entry['project_root'] is not None or classification['recommendation'] == 'leave_untouched' or any(_excluded(p) for p in path.split('/')):
            untouched.append(dict(path=path, reason='protected_or_non_file'))
            continue
        category = classification['category']
        preferred = preferences.destination(scope, category) if preferences else None
        candidates = rank_destinations(path, category, folders, preferred=preferred)
        destination = str(PurePosixPath(candidates[0]['path']) / PurePosixPath(path).name) if candidates else None
        reason = 'incomplete_inventory' if incomplete else 'no_destination_evidence' if not candidates else None
        if destination == path:
            untouched.append(dict(path=path, reason='already_in_destination'))
            continue
        if destination and collision_key(destination) in occupied:
            reason = 'destination_collision'
        if destination and reason is None:
            occupied.add(collision_key(destination))
        permission = decide('move', settings, authorized=False, safe=False, certain=False,
                            proposal_revision=settings.revision)
        identity = hashlib.sha256((path + '\0' + (destination or '')).encode()).hexdigest()[:24]
        proposals.append(dict(id=identity, source=path, destination=destination, category=category,
            action='move', reason=reason or 'advisory_destination_ranking', candidates=candidates,
            review_required=True, blocked=reason is not None, executable=False,
            source_snapshot=dict(size_bytes=entry['size_bytes'], mtime_ns=entry['mtime_ns']),
            policy=permission))
    fingerprint = hashlib.sha256(json.dumps(report, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    return dict(schema_version='0.1.0', report_type='organization_plan', root=report['root'],
                read_only=True, executable=False, inventory_sha256=fingerprint,
                policy_revision=settings.revision, mode=settings.mode.value,
                ranking_method='lexical_category_baseline', scores_are_probabilities=False,
                proposals=proposals, untouched=untouched,
                summary=dict(proposals=len(proposals), untouched=len(untouched),
                             blocked=sum(p['blocked'] for p in proposals)),
                limitations=['Advisory report only', 'Filesystem identity and permissions must be rechecked',
                             'No semantic model or independent destination benchmark'])


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('report', help='Explicit scanner report JSON')
    parser.add_argument('folders', help='Explicit candidate folder JSON')
    args = parser.parse_args(argv)
    try:
        # These inputs are explicitly chosen app reports rather than discovered documents
        from pathlib import Path
        for path in (args.report, args.folders):
            if Path(path).is_symlink() or Path(path).stat().st_size > 8 * 1024 * 1024:
                raise ValueError('Unsafe or oversized input report')
        result = build_plan(json.loads(Path(args.report).read_text()), json.loads(Path(args.folders).read_text()))
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(json.dumps(dict(error=type(exc).__name__)), file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2))
    return 0


if __name__ == '__main__':
    sys.exit(main())

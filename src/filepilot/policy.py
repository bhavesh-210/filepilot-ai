"""Pure permission decisions independent of classifiers and filesystem execution"""
from dataclasses import dataclass
from enum import Enum


class Mode(str, Enum):
    FULL = 'full'
    SMART = 'smart'
    LIMITED = 'limited'


@dataclass(frozen=True)
class PolicySettings:
    mode: Mode = Mode.FULL
    rename_enabled: bool = True
    revision: int = 0

    def __post_init__(self):
        if not isinstance(self.mode, Mode) or type(self.rename_enabled) is not bool:
            raise ValueError('Invalid policy settings')
        if type(self.revision) is not int or self.revision < 0:
            raise ValueError('Invalid policy revision')


def decide(action, settings, *, authorized=False, safe=False, certain=False,
           approved=False, rule_allowed=False, separately_confirmed=False,
           proposal_revision=None):
    """Reevaluate with current settings immediately before any future execution"""
    if not isinstance(settings, PolicySettings):
        raise ValueError('Invalid settings')
    flags = (authorized, safe, certain, approved, rule_allowed, separately_confirmed)
    if any(type(value) is not bool for value in flags):
        raise ValueError('Permission inputs must be boolean')
    if action not in {'move', 'rename', 'trash', 'delete'}:
        return dict(allowed=False, decision='blocked', reason='unsupported_action')
    if not authorized or not safe:
        return dict(allowed=False, decision='blocked', reason='authorization_or_safety_failed')
    if type(proposal_revision) is not int or proposal_revision != settings.revision:
        return dict(allowed=False, decision='review', reason='policy_changed_or_missing_revision')
    if action == 'rename' and not settings.rename_enabled:
        return dict(allowed=False, decision='blocked', reason='renaming_disabled')
    if action in {'trash', 'delete'} and not separately_confirmed:
        return dict(allowed=False, decision='review', reason='separate_confirmation_required')
    if not certain and not approved:
        return dict(allowed=False, decision='review', reason='uncertain_recommendation')
    if settings.mode == Mode.SMART and not approved:
        return dict(allowed=False, decision='review', reason='approval_required')
    if settings.mode == Mode.LIMITED and not (approved or rule_allowed):
        return dict(allowed=False, decision='review', reason='outside_approved_rules')
    return dict(allowed=True, decision='permitted', reason='current_policy_allows')

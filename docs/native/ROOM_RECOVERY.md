# Public Room recovery checkpoints

The host can explicitly save a new public recovery checkpoint and later recover it
from Network Room controls. This is recovery to the last saved checkpoint, not an
automatic journal: later changes, pending requests and in-flight actions are lost.

## Saved state and restart

- Saves the Room name, host authority identity, current member identities/names,
  shared tasks/dependencies/checklists, handoffs, evidence, retained activity and
  public messages, readiness/impacts/help, milestones, Ship scope/reports and the
  remaining countdown. Existing bounded retention limits still apply.
- Excludes DMs, saved patch bodies/receipts, reactions, TLS keys/certificates,
  invites, session tokens, source files, terminal/AI state and live focus/location
  sharing. These are not restored. The countdown is paused on recovery.
- Files are explicitly selected, bounded to 8 MiB, versioned and validated before
  starting the recovered host. Saving uses create-only access and flushes the file;
  it never replaces a previous checkpoint. An interrupted save can leave an invalid
  new file; retain the previous checkpoint. Invalid files remain untouched.
- Recovery creates a fresh TLS identity and no sessions or grants. Active and
  paused Rooms restore paused; archived Rooms remain terminally read-only.
  Review the restored state before resuming. Manual reports remain declarations,
  not fresh verification of checks or a provider result.

## Owner continuity

Recovered members are offline. Ordinary fresh invites create new identities even
when display names match saved owners. The local host can issue a fresh expiring,
single-use reclaim invite bound to one recovered identity, or explicitly rejoin
locally as that owner. Give a reclaim invite privately to the original owner: its
bearer receives that identity's task authority. This is host-approved recovery,
not account authentication or proof that a display name identifies a person.

Reissuing a claim revokes earlier outstanding claims for that identity. Existing
invite revocation also ejects a reclaimed session. Already-reclaimed identities
cannot be claimed again. Old sessions and invites do not survive restart, even at
the same address. Offline owners retain their task assignments while awaiting a
fresh claim. Only the host has these local recovery controls.

## Acceptance boundaries

Independent-process tests cover restart at the same address, public/private
projection, create-only preservation, paused recovery, owner continuity, display
name impersonation denial, stale claims and revocation. Unit tests cover terminal
archive recovery and unsupported, credential-injected and truncated files.

Desktop button/dialog behavior and physical two-machine restart/reclaim remain
separate manual gates. This feature does not provide an automatic crash journal,
private-history recovery, account recovery, internet relay or host migration.

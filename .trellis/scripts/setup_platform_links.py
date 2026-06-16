#!/usr/bin/env python3
"""
Cross-platform Trellis platform link setup.

Creates the appropriate links (junctions on Windows, symlinks on Unix)
so that shared Trellis agents/skills/commands are accessible from each
AI platform's config directory without duplicating files in git.

Usage:
    python .trellis/scripts/setup_platform_links.py

Links created (all point to .opencode/ equivalents):
    .claude/agents/      -> .opencode/agents/
    .claude/skills/      -> .opencode/skills/
    .claude/commands/    -> .opencode/commands/
    .codex/skills/       -> .opencode/skills/
    .codex/commands/     -> .opencode/commands/
    .agents/skills/      -> .opencode/skills/   (shared skill layer)

Idempotent: safe to run multiple times.
"""
import os
import sys
import platform
import subprocess
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
OPENDCODE_DIR = PROJECT_ROOT / ".opencode"

# Links to create: (link_path, target_relative_to_project_root)
LINKS = [
    (".claude/agents", ".opencode/agents"),
    (".claude/skills", ".opencode/skills"),
    (".claude/commands", ".opencode/commands"),
    (".codex/skills", ".opencode/skills"),
    (".codex/commands", ".opencode/commands"),
    (".agents/skills", ".opencode/skills"),
]

IS_WINDOWS = platform.system() == "Windows"


def remove_existing(path: Path):
    """Remove an existing directory, junction, or symlink at path."""
    if not path.exists():
        return
    if path.is_symlink() or path.is_junction():
        # On Windows, rmdir works for junctions; unlink for symlinks
        if IS_WINDOWS:
            subprocess.run(
                ["cmd", "/c", "rmdir", str(path.absolute())],
                capture_output=True,
            )
        else:
            path.unlink()
    elif path.is_dir():
        # Safety check: only remove empty directories
        try:
            path.rmdir()
        except OSError:
            print(f"  [WARN] {path} is a non-empty directory — skipping removal")
            return
    print(f"  Removed existing: {path}")


def create_link(link_path: Path, target_path: Path):
    """Create a junction (Windows) or symlink (Unix)."""
    link_parent = link_path.parent
    link_parent.mkdir(parents=True, exist_ok=True)

    if IS_WINDOWS:
        # Use PowerShell New-Item -Type Junction (no admin required)
        result = subprocess.run(
            [
                "powershell",
                "-Command",
                f"New-Item -Path '{link_path}' -ItemType Junction "
                f"-Target '{target_path}' -ErrorAction Stop",
            ],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(f"  [ERROR] Failed to create junction: {link_path}")
            print(f"    {result.stderr.strip()}")
            return False
    else:
        # Unix: create relative symlink
        link_path.symlink_to(
            os.path.relpath(target_path, link_parent),
            target_is_directory=True,
        )

    print(f"  Created: {link_path} -> {target_path}")
    return True


def main():
    print("Trellis Platform Link Setup")
    print(f"  Platform: {platform.system()}")
    print(f"  Project:  {PROJECT_ROOT}")
    print()

    if not OPENDCODE_DIR.is_dir():
        print("[ERROR] .opencode/ directory not found.")
        print("This project must have .opencode/ configured first.")
        sys.exit(1)

    success = 0
    skipped = 0

    for link_rel, target_rel in LINKS:
        link_path = PROJECT_ROOT / link_rel
        target_path = PROJECT_ROOT / target_rel

        # Resolve the target to an absolute path (required for Windows junctions)
        target_abs = target_path.resolve().absolute()

        if link_path.exists():
            # Check if it already points to the right place
            if link_path.is_symlink() or (IS_WINDOWS and _is_junction(link_path)):
                resolved = link_path.resolve()
                if resolved == target_abs:
                    print(f"  [OK] {link_rel} (already correct)")
                    skipped += 1
                    continue
            # Wrong target or regular directory — remove and recreate
            remove_existing(link_path)

        if create_link(link_path, target_abs):
            success += 1

    print()
    print(f"Done: {success} created, {skipped} already correct.")

    # Verify
    print()
    print("Verification:")
    for link_rel, _ in LINKS:
        link_path = PROJECT_ROOT / link_rel
        if link_path.exists():
            try:
                contents = list(link_path.iterdir())
                names = ", ".join(p.name for p in contents[:3])
                more = "..." if len(contents) > 3 else ""
                print(f"  [OK] {link_rel}/ -> {names}{more}")
            except Exception:
                print(f"  [FAIL] {link_rel}/ — cannot read")
        else:
            print(f"  [MISS] {link_rel}/ — not found")


def _is_junction(path: Path) -> bool:
    """Check if a Windows path is a directory junction."""
    if not IS_WINDOWS:
        return False
    # On Windows, junctions appear as symlinks to Python's pathlib
    # But Path.is_junction() is only available in Python 3.12+
    try:
        return bool(path.is_junction())
    except AttributeError:
        # Fallback for Python < 3.12
        result = subprocess.run(
            ["cmd", "/c", f"dir /AL {path.parent} 2>nul | findstr {path.name}"],
            capture_output=True,
            text=True,
        )
        return "<JUNCTION>" in result.stdout


if __name__ == "__main__":
    main()

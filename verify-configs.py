#!/usr/bin/env python3

import sys
from pathlib import Path
import xml.etree.ElementTree as ET

try:
    import yaml
except ImportError:
    print("Error: PyYAML not installed. Install with: pip install pyyaml")
    sys.exit(1)


class ConfigVerifier:
    def __init__(self):
        self.home = Path.home()
        self.cemu_config = self.home / ".config" / "Cemu" / "settings.xml"
        self.ukmm_config = self.home / ".config" / "ukmm" / "settings.yml"
        self.errors = 0

    def load_cemu_config(self):
        """Load and parse Cemu configuration."""
        if not self.cemu_config.exists():
            print(f"❌ Cemu config not found: {self.cemu_config}")
            sys.exit(1)

        tree = ET.parse(self.cemu_config)
        root = tree.getroot()

        # Extract game paths from GamePaths
        game_paths = []
        for entry in root.findall(".//GamePaths/Entry"):
            if entry.text:
                game_paths.append(entry.text)

        return {"game_paths": game_paths}

    def load_ukmm_config(self):
        """Load and parse UKMM configuration."""
        if not self.ukmm_config.exists():
            print(f"❌ UKMM config not found: {self.ukmm_config}")
            sys.exit(1)

        with open(self.ukmm_config, 'r') as f:
            config = yaml.safe_load(f)

        # Extract Wii U dump configuration
        wiiu_config = config.get('wiiu_config', {})
        dump = wiiu_config.get('dump', {})
        source = dump.get('source', {})

        # Extract deployment configuration
        deploy_config = wiiu_config.get('deploy_config')

        return {
            "host_path": source.get('host_path'),
            "content_dir": source.get('content_dir'),
            "update_dir": source.get('update_dir'),
            "aoc_dir": source.get('aoc_dir'),
            "deploy_config": deploy_config,
        }

    def check_path_exists(self, path, name):
        """Verify a path exists and is a directory."""
        p = Path(path) if not isinstance(path, Path) else path
        if p.is_dir():
            print(f"  ✓ {name}")
            return True
        else:
            print(f"  ❌ {name} NOT FOUND: {p}")
            self.errors += 1
            return False

    def check_file_exists(self, path, name):
        """Verify a file exists."""
        p = Path(path) if not isinstance(path, Path) else path
        if p.is_file():
            print(f"    ✓ {name}")
            return True
        else:
            print(f"    ❌ {name} NOT FOUND: {p}")
            self.errors += 1
            return False

    def run(self):
        """Run all verifications."""
        print("=== Configuration Verification ===\n")

        # Load configurations
        cemu_cfg = self.load_cemu_config()
        ukmm_cfg = self.load_ukmm_config()

        print("✓ Both config files found\n")

        # Display Cemu configuration
        print("=== Cemu Configuration ===")
        for i, path in enumerate(cemu_cfg["game_paths"], 1):
            print(f"Game Path {i}: {path}")
        print()

        # Display UKMM configuration
        print("=== UKMM Dump Configuration ===")
        print(f"Host Path:    {ukmm_cfg['host_path']}")
        print(f"Content Dir:  {ukmm_cfg['content_dir']}")
        print(f"Update Dir:   {ukmm_cfg['update_dir']}")
        print(f"AOC Dir:      {ukmm_cfg['aoc_dir']}")
        print()

        print("=== UKMM Deployment Configuration ===")
        if ukmm_cfg['deploy_config']:
            print(f"Deploy Path:   {ukmm_cfg['deploy_config'].get('deployment_folder')}")
            print(f"Deploy Layout: {ukmm_cfg['deploy_config'].get('deployment_layout')}")
            print(f"Deploy Method: {ukmm_cfg['deploy_config'].get('deployment_method')}")
        else:
            print("⚠ No deployment configuration set")
        print()

        # Verify paths exist
        print("=== Path Verification ===")
        cemu_path_valid = False
        if cemu_cfg["game_paths"]:
            cemu_path_valid = self.check_path_exists(
                cemu_cfg["game_paths"][0],
                "Cemu game path"
            )

        self.check_path_exists(ukmm_cfg['host_path'], "UKMM host path")
        self.check_path_exists(ukmm_cfg['content_dir'], "UKMM content dir")
        self.check_path_exists(ukmm_cfg['update_dir'], "UKMM update dir")
        self.check_path_exists(ukmm_cfg['aoc_dir'], "UKMM DLC dir")
        print()

        # Verify game dump structure
        print("=== Game Dump Verification ===")
        content_dir = Path(ukmm_cfg['content_dir'])
        if content_dir.is_dir():
            self.check_file_exists(
                content_dir / "Pack" / "Dungeon001.pack",
                "Base game Dungeon001.pack"
            )

        aoc_dir = Path(ukmm_cfg['aoc_dir'])
        if aoc_dir.is_dir():
            self.check_file_exists(
                aoc_dir / "Pack" / "AocMainField.pack",
                "DLC AocMainField.pack"
            )
        print()

        # Check deployment configuration
        print("=== Deployment Verification ===")
        expected_deploy_path = Path(self.home, ".local", "share", "Cemu", "graphicPacks")

        deploy_config = ukmm_cfg['deploy_config']
        if deploy_config:
            deploy_path = Path(deploy_config.get('output'))

            if deploy_path == expected_deploy_path:
                print("✓ Deployment path points to Cemu graphics packs")
            else:
                print("⚠ Deployment path is not standard Cemu location:")
                print(f"  Expected: {expected_deploy_path}")
                print(f"  Actual:   {deploy_path}")

            deploy_layout = deploy_config.get('layout')
            if deploy_layout == "WithName":
                print("✓ Deployment layout is 'WithName' (correct for Cemu)")
            else:
                print(f"❌ Deployment layout is '{deploy_layout}' (should be 'WithName')")
                self.errors += 1

            if deploy_path.exists():
                print("✓ Deployment path exists")
            else:
                print(f"❌ Deployment path does not exist: {deploy_path}")
                self.errors += 1
        else:
            print("❌ No deployment configuration set")
            self.errors += 1

        print()

        # Check alignment between Cemu and UKMM
        print("=== Configuration Alignment ===")
        if cemu_path_valid:
            cemu_path = Path(cemu_cfg["game_paths"][0])
            ukmm_path = Path(ukmm_cfg['host_path'])

            if cemu_path == ukmm_path:
                print("✓ Cemu and UKMM point to the same root directory")
            else:
                print("⚠ Cemu and UKMM have different root paths:")
                print(f"  Cemu: {cemu_path}")
                print(f"  UKMM: {ukmm_path}")
        print()

        # Final status
        if self.errors == 0:
            print("=== All checks passed! ===")
            return 0
        else:
            print(f"=== {self.errors} error(s) found ===")
            return 1


if __name__ == "__main__":
    verifier = ConfigVerifier()
    sys.exit(verifier.run())

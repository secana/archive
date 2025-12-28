{
  description = "Development and build environment for the archive Rust crate";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    ,
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ] (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Archive creation tools
            zip
            unzip
            gzip
            bzip2
            xz
            zstd
            gnutar
            p7zip
            lzip
            lz4

            # Additional utilities
            tree # For viewing directory structure
            file # For file type identification
            coreutils # For basic file operations

            # Rust development tools
            rustc
            cargo
            rustfmt
            clippy

            # Git for version control
            git
          ];

          shellHook = ''
            echo "Archive creation environment loaded!"
            echo "Available tools:"
            echo "  - zip/unzip"
            echo "  - gzip/gunzip"
            echo "  - bzip2/bunzip2"
            echo "  - xz/unxz"
            echo "  - tar"
            echo "  - zstd"
            echo "  - 7z"
            echo "  - lzip/lz4"
            echo ""
            echo "Run 'nix run .#generateTestArchives' to create all test archives"
          '';
        };

        packages.generate-archives = pkgs.writeShellScriptBin "generate-test-archives" ''
          set -e

          # Add all required tools to PATH
          export PATH="${
            pkgs.lib.makeBinPath [
              pkgs.zip
              pkgs.unzip
              pkgs.gzip
              pkgs.bzip2
              pkgs.xz
              pkgs.zstd
              pkgs.gnutar
              pkgs.p7zip
              pkgs.lzip
              pkgs.lz4
              pkgs.tree
              pkgs.file
              pkgs.coreutils
            ]
          }:$PATH"

          SCRIPT_DIR="$(cd "$(dirname "''${BASH_SOURCE[0]}")" && pwd)"
          TEST_DIR="''${1:-test-archives}"

          echo "Creating test archives in: $TEST_DIR"
          mkdir -p "$TEST_DIR"
          cd "$TEST_DIR"

          # Create test data directory structure
          echo "Creating test data..."
          mkdir -p test-data/{empty-dir,nested/deep/path}

          # Create various test files
          echo "Hello, World!" > test-data/hello.txt
          echo "This is a test file" > test-data/test.txt
          dd if=/dev/urandom of=test-data/binary.bin bs=1024 count=10 2>/dev/null
          echo "Nested file content" > test-data/nested/file.txt
          echo "Deep nested content" > test-data/nested/deep/path/deep-file.txt

          # Create a larger file for compression testing
          dd if=/dev/urandom of=test-data/large-file.bin bs=1M count=1 2>/dev/null

          echo ""
          echo "=== Creating ZIP archives ==="

          # Basic ZIP
          echo "Creating: basic.zip"
          zip -r basic.zip test-data/ >/dev/null

          # ZIP with different compression levels
          echo "Creating: no-compression.zip (store only)"
          zip -r -0 no-compression.zip test-data/ >/dev/null

          echo "Creating: max-compression.zip"
          zip -r -9 max-compression.zip test-data/ >/dev/null

          # ZIP with password (for testing encrypted archives)
          echo "Creating: encrypted.zip (password: test123)"
          zip -r -P test123 encrypted.zip test-data/ >/dev/null

          echo ""
          echo "=== Creating TAR archives ==="

          # Plain TAR
          echo "Creating: archive.tar"
          tar -cf archive.tar test-data/

          echo ""
          echo "=== Creating TAR archives ==="

          # AR (Unix Archive, can only store files and not directories)
          echo "Creating: archive.ar"
          ar rc archive.ar $(find test-data -type f)

          echo ""
          echo "=== Creating TAR.GZ archives ==="

          # TAR.GZ (gzip)
          echo "Creating: archive.tar.gz"
          tar -czf archive.tar.gz test-data/

          # Alternative naming
          echo "Creating: archive.tgz"
          tar -czf archive.tgz test-data/

          echo ""
          echo "=== Creating TAR.BZ2 archives ==="

          # TAR.BZ2 (bzip2)
          echo "Creating: archive.tar.bz2"
          tar -cjf archive.tar.bz2 test-data/

          # Alternative naming
          echo "Creating: archive.tbz2"
          tar -cjf archive.tbz2 test-data/

          echo ""
          echo "=== Creating TAR.XZ archives ==="

          # TAR.XZ (xz)
          echo "Creating: archive.tar.xz"
          tar -cJf archive.tar.xz test-data/

          # Alternative naming
          echo "Creating: archive.txz"
          tar -cJf archive.txz test-data/

          echo ""
          echo "=== Creating TAR.ZST archives ==="

          # TAR.ZST (zstd)
          echo "Creating: archive.tar.zst"
          tar -c test-data/ | zstd -q -o archive.tar.zst

          echo ""
          echo "=== Creating compressed single files ==="

          # GZIP single file
          echo "Creating: hello.txt.gz"
          gzip -c test-data/hello.txt > hello.txt.gz

          # BZIP2 single file
          echo "Creating: hello.txt.bz2"
          bzip2 -c test-data/hello.txt > hello.txt.bz2

          # XZ single file
          echo "Creating: hello.txt.xz"
          xz -c test-data/hello.txt > hello.txt.xz

          # ZSTD single file
          echo "Creating: hello.txt.zst"
          zstd -q test-data/hello.txt -o hello.txt.zst

          # LZ4 single file
          echo "Creating: hello.txt.lz4"
          lz4 -q test-data/hello.txt hello.txt.lz4

          echo ""
          echo "=== Creating 7z archives ==="

          # 7z archive
          echo "Creating: archive.7z"
          7z a -bd archive.7z test-data/ >/dev/null

          echo ""
          echo "=== Creating nested archives ==="

          # Create a nested archive structure
          mkdir -p nested-test
          cp basic.zip nested-test/
          cp archive.tar.gz nested-test/

          echo "Creating: nested.zip (contains other archives)"
          zip -r nested.zip nested-test/ >/dev/null

          echo "Creating: nested.tar.gz (contains other archives)"
          tar -czf nested.tar.gz nested-test/

          # Deep nesting (3 levels)
          mkdir -p level3
          echo "Level 3 content" > level3/level3.txt
          zip -r level3.zip level3/ >/dev/null

          mkdir -p level2
          cp level3.zip level2/
          echo "Level 2 content" > level2/level2.txt
          zip -r level2.zip level2/ >/dev/null

          mkdir -p level1
          cp level2.zip level1/
          echo "Level 1 content" > level1/level1.txt
          echo "Creating: deeply-nested.zip (3 levels deep)"
          zip -r deeply-nested.zip level1/ >/dev/null

          echo ""
          echo "=== Creating edge case archives ==="

          # Empty archive - create a proper empty ZIP file
          echo "Creating: empty.zip"
          touch .empty_placeholder
          zip empty.zip .empty_placeholder >/dev/null
          zip -d empty.zip .empty_placeholder >/dev/null
          rm .empty_placeholder

          # Archive with only empty directories
          mkdir -p empty-dirs/{dir1,dir2/subdir}
          echo "Creating: empty-dirs.zip"
          zip -r empty-dirs.zip empty-dirs/ >/dev/null

          # Archive with special characters in names
          mkdir -p special-chars
          echo "test" > "special-chars/file with spaces.txt"
          echo "test" > "special-chars/file-with-ümlaut.txt"
          echo "Creating: special-chars.zip"
          zip -r special-chars.zip special-chars/ >/dev/null

          # Large file for bomb detection testing
          echo "Creating: potential-bomb.zip (highly compressible)"
          dd if=/dev/zero of=zeros.bin bs=1M count=10 2>/dev/null
          zip -9 potential-bomb.zip zeros.bin >/dev/null

          echo ""
          echo "=== Cleaning up temporary directories ==="
          rm -rf test-data nested-test level1 level2 level3 empty-dirs special-chars zeros.bin

          echo ""
          echo "=== Test Archive Summary ==="
          echo "Created archives:"
          tree -h -L 1 --filesfirst

          echo ""
          echo "=== Creating manifest ==="
          cat > MANIFEST.md << 'EOF'
          # Test Archives Manifest

          This directory contains reproducible test archives for testing the archive Rust crate.

          ## Archive Types

          ### ZIP Archives
          - \`basic.zip\` - Standard ZIP archive with test files
          - \`no-compression.zip\` - ZIP with store method (no compression)
          - \`max-compression.zip\` - ZIP with maximum compression
          - \`encrypted.zip\` - Password-protected ZIP (password: test123)
          - \`nested.zip\` - ZIP containing other archives
          - \`deeply-nested.zip\` - ZIP with 3 levels of nesting
          - \`empty.zip\` - Empty ZIP archive (no files)
          - \`empty-dirs.zip\` - ZIP containing only empty directories
          - \`special-chars.zip\` - ZIP with special characters in filenames

          ### TAR Archives
          - \`archive.tar\` - Plain TAR archive

          ### Compressed TAR Archives
          - \`archive.tar.gz\` / \`archive.tgz\` - TAR with gzip compression
          - \`archive.tar.bz2\` / \`archive.tbz2\` - TAR with bzip2 compression
          - \`archive.tar.xz\` / \`archive.txz\` - TAR with xz compression
          - \`archive.tar.zst\` - TAR with zstd compression
          - \`nested.tar.gz\` - Compressed TAR containing other archives

          ### Single File Compression
          - \`hello.txt.gz\` - gzip compressed file
          - \`hello.txt.bz2\` - bzip2 compressed file
          - \`hello.txt.xz\` - xz compressed file
          - \`hello.txt.zst\` - zstd compressed file
          - \`hello.txt.lz4\` - lz4 compressed file

          ### Other Formats
          - \`archive.7z\` - 7-Zip archive

          ### Edge Cases
          - \`potential-bomb.zip\` - Highly compressible data (10MB of zeros)

          ## Test Data Structure

          Original test data structure before archiving:
          \`\`\`
          test-data/
          ├── empty-dir/
          ├── nested/
          │   ├── file.txt
          │   └── deep/
          │       └── path/
          │           └── deep-file.txt
          ├── hello.txt
          ├── test.txt
          ├── binary.bin (10KB random data)
          └── large-file.bin (1MB random data)
          \`\`\`

          ## Checksums

          To verify archive integrity, you can generate checksums:
          \`\`\`bash
          sha256sum *.zip *.tar* *.gz *.bz2 *.xz *.zst *.lz4 *.7z > checksums.txt
          \`\`\`

          ## Regeneration

          To regenerate all archives:
          \`\`\`bash
          nix run .#generateTestArchives
          \`\`\`

          Or with a custom output directory:
          \`\`\`bash
          nix run .#generateTestArchives -- /path/to/output
          \`\`\`
          EOF

          echo "Created MANIFEST.md"

          echo ""
          echo "✅ All test archives created successfully!"
          echo ""
          echo "Total archives created: $(ls -1 *.zip *.tar* *.7z *.gz *.bz2 *.xz *.zst *.lz4 2>/dev/null | wc -l | tr -d ' ')"
          echo ""
          echo "To verify archives, you can use:"
          echo "  unzip -t *.zip"
          echo "  tar -tzf *.tar.gz"
          echo "  tar -tjf *.tar.bz2"
          echo "  tar -tJf *.tar.xz"
        '';

        apps = {
          default = {
            type = "app";
            program = "${self.packages.${system}.generate-archives}/bin/generate-test-archives";
          };

          generateTestArchives = {
            type = "app";
            program = "${self.packages.${system}.generate-archives}/bin/generate-test-archives";
          };
        };
      }
    );
}

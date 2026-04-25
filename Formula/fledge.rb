class Fledge < Formula
  desc "Corvid-themed project scaffolding CLI — get your projects ready to fly"
  homepage "https://github.com/CorvidLabs/fledge"
  license "MIT"
  # NOTE: This file is updated POST-release by .github/workflows/post-release-formula.yml
  # — once release.yml uploads the binaries and their .sha256 sidecars, that
  # workflow opens a PR bumping the version and shas together. Don't try to bump
  # this manually during `fledge release` (the new shas don't exist at bump time).
  version "0.15.3"

  on_macos do
    on_arm do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-macos-aarch64"
      sha256 "40059427ec81431b8ce7683568b2c97ff429aaf94f68fcbb1a75f94c7f25fadc"

      def install
        bin.install "fledge-macos-aarch64" => "fledge"
      end
    end

    on_intel do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-macos-x86_64"
      sha256 "4158e51ec49b9c0970fc50bec1cb47953b150b55c8a1ea76d518d5cfcb23b5b0"

      def install
        bin.install "fledge-macos-x86_64" => "fledge"
      end
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-linux-x86_64"
      sha256 "84c9d86bf21c9cec5b43bcab1b42f40f043fb0f5db55d4b374093b7a4212bb03"

      def install
        bin.install "fledge-linux-x86_64" => "fledge"
      end
    end
  end

  def caveats
    <<~EOS
      To generate shell completions:
        fledge completions bash > $(brew --prefix)/etc/bash_completion.d/fledge
        fledge completions zsh > $(brew --prefix)/share/zsh/site-functions/_fledge
        fledge completions fish > $(brew --prefix)/share/fish/vendor_completions.d/fledge.fish
    EOS
  end

  test do
    assert_match "fledge", shell_output("#{bin}/fledge --version")
  end
end

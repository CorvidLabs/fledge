class Fledge < Formula
  desc "Corvid-themed project scaffolding CLI — get your projects ready to fly"
  homepage "https://github.com/CorvidLabs/fledge"
  license "MIT"
  # NOTE: This file is updated POST-release by .github/workflows/post-release-formula.yml
  # — once release.yml uploads the binaries and their .sha256 sidecars, that
  # workflow opens a PR bumping the version and shas together. Don't try to bump
  # this manually during `fledge release` (the new shas don't exist at bump time).
  version "1.0.1"

  on_macos do
    on_arm do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-macos-aarch64"
      sha256 "6c840cf2bf687340d6b28aad45bc9eb793f716fe39d73e98a6dbb82315b7c5c0"

      def install
        bin.install "fledge-macos-aarch64" => "fledge"
      end
    end

    on_intel do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-macos-x86_64"
      sha256 "403617416fe5df59bb044ef38ef6f7b5ebebeb1c6bec2c28519ef6299563b398"

      def install
        bin.install "fledge-macos-x86_64" => "fledge"
      end
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-linux-x86_64"
      sha256 "6acd2c2bdd6792189da2c9b59cea70e0a43815cbd93226e7fb64eca89cb4bb5d"

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

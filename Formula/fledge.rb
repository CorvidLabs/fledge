class Fledge < Formula
  desc "Corvid-themed project scaffolding CLI — get your projects ready to fly"
  homepage "https://github.com/CorvidLabs/fledge"
  license "MIT"
  # NOTE: This file is updated POST-release by .github/workflows/post-release-formula.yml
  # — once release.yml uploads the binaries and their .sha256 sidecars, that
  # workflow opens a PR bumping the version and shas together. Don't try to bump
  # this manually during `fledge release` (the new shas don't exist at bump time).
  version "0.17.0"

  on_macos do
    on_arm do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-macos-aarch64"
      sha256 "1c8f09dabf479fd69052cce4bf626e4ef5d79aaa45810fa477c546ca231d31c2"

      def install
        bin.install "fledge-macos-aarch64" => "fledge"
      end
    end

    on_intel do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-macos-x86_64"
      sha256 "46871667b599acbe8ea82e18029df32a6702959ace9110dac7a41c0710a2a6ea"

      def install
        bin.install "fledge-macos-x86_64" => "fledge"
      end
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-linux-x86_64"
      sha256 "b8775256d34fbdf4625eaa2c59550f42879aedb7821ea22be9dd773d3329ad95"

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

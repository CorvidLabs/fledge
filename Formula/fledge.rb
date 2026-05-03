class Fledge < Formula
  desc "Corvid-themed project scaffolding CLI — get your projects ready to fly"
  homepage "https://github.com/CorvidLabs/fledge"
  license "MIT"
  # NOTE: This file is updated POST-release by .github/workflows/post-release-formula.yml
  # — once release.yml uploads the binaries and their .sha256 sidecars, that
  # workflow opens a PR bumping the version and shas together. Don't try to bump
  # this manually during `fledge release` (the new shas don't exist at bump time).
  version "1.1.0"

  on_macos do
    on_arm do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-macos-aarch64"
      sha256 "0e28df99b72c25bb84be70bd4c4c2ef51ec4da09830fadf33c1ee66b1241329d"

      def install
        bin.install "fledge-macos-aarch64" => "fledge"
      end
    end

    on_intel do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-macos-x86_64"
      sha256 "f03b665f6f765371f5293aece1b46ec6e5d325b365b21165245e0e5644ad149b"

      def install
        bin.install "fledge-macos-x86_64" => "fledge"
      end
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-linux-x86_64"
      sha256 "5705ea25a5c985091f0e81bb80c54cfbfab934578156def03d43fd6816ba3b0c"

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

class Fledge < Formula
  desc "Corvid-themed project scaffolding CLI — get your projects ready to fly"
  homepage "https://github.com/CorvidLabs/fledge"
  license "MIT"
  # NOTE: This file is updated POST-release by .github/workflows/post-release-formula.yml
  # — once release.yml uploads the binaries and their .sha256 sidecars, that
  # workflow opens a PR bumping the version and shas together. Don't try to bump
  # this manually during `fledge release` (the new shas don't exist at bump time).
  version "0.16.0"

  on_macos do
    on_arm do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-macos-aarch64"
      sha256 "467b17217c6bc9b6ac9b7df550c4e5f1cc823a483e9ad2871b589bb8cf33ea9b"

      def install
        bin.install "fledge-macos-aarch64" => "fledge"
      end
    end

    on_intel do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-macos-x86_64"
      sha256 "a51c57dc60bc7753eba2b4a8017a88715543ad10807893b4090229a2bb0e3e87"

      def install
        bin.install "fledge-macos-x86_64" => "fledge"
      end
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/CorvidLabs/fledge/releases/download/v#{version}/fledge-linux-x86_64"
      sha256 "3285a92cd016791a792d056ef0184e9bdba8ce3c0353e41a105897547bba7e1c"

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

# Documentation: https://docs.brew.sh/Formula-Cookbook
#                https://rubydoc.brew.sh/Formula
class Claudectx < Formula
  desc "Launch Claude Code with different profiles"
  homepage "https://github.com/FGRibreau/claudectx"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/FGRibreau/claudectx/releases/download/v#{version}/claudectx_darwin_aarch64.tar.gz"
      sha256 "a273317f9e98c5d4b7dc05efc40acd8f914dbaf7164f7701f03ecfb83c4bfc2f"
    end
    on_intel do
      url "https://github.com/FGRibreau/claudectx/releases/download/v#{version}/claudectx_darwin_x86_64.tar.gz"
      sha256 "68de4100c10fca876ce2be6cb4d521b18243907f6476a35d85b154f82e3c9c8f"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/FGRibreau/claudectx/releases/download/v#{version}/claudectx_linux_aarch64.tar.gz"
      sha256 "0e61b02f861b7040ee6cc100eb27cb671db95e86623386637c4ea41484f5bc33"
    end
    on_intel do
      url "https://github.com/FGRibreau/claudectx/releases/download/v#{version}/claudectx_linux_x86_64.tar.gz"
      sha256 "09ac37e6b7842a0dd18cfbf47ff849e0e463cab65deeca0ba6b1af8dc29a3134"
    end
  end

  def install
    bin.install "claudectx"
  end

  test do
    assert_match "claudectx", shell_output("#{bin}/claudectx --version")
  end
end

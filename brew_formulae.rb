class PpUtil < Formula
    desc "display information about installed provisioning profiles"
    homepage "https://github.com/juwens/pputil"
    url "https://github.com/juwens/pputil/archive/refs/tags/v1.1.1.tar.gz"
    sha256 "todo"
    license "Mozilla"

    bottle do
        # examples from dua-cli
        #sha256 cellar: :any_skip_relocation, arm64_sequoia: "5fb9a5e8a7846b065f434c6af687eeb6ddf4bf9daea76fb86b518efe16245f9f"
        #sha256 cellar: :any_skip_relocation, arm64_sonoma:  "de2c481f23fe2bbb2c84f1fc4cec4e06e3a41a3e4c0d4384c23592874ac6507e"
        #sha256 cellar: :any_skip_relocation, arm64_ventura: "945220d400a55a19dabc8fc3a3b55f9c02f029f07980ab217bcea15e829f2f12"
        #sha256 cellar: :any_skip_relocation, sonoma:        "6c332874c489ad073c62fde95263f7f5bc94c596cc35ac71b28b272ca67110f4"
        #sha256 cellar: :any_skip_relocation, ventura:       "35e0d45677d56d43cea04c0e903b1348a6751d99899c72848734845569f76748"
        #sha256 cellar: :any_skip_relocation, x86_64_linux:  "57001359dbe89691b5462a320a27dfcb58c0535f00ce08928830507efe5c67af"
      end
    
      depends_on "rust" => :build

      def install
        system "cargo", "install", *std_cargo_args
      end
end
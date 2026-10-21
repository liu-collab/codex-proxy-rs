# 从构建声明读取 CI 工具链，阻止 Docker 与本地 Rust 版本漂移
module BuildToolchains
  def self.read(root = '.')
    declaration = File.read(File.join(root, 'backend/rust-toolchain.toml'))
    rust = declaration[/^\s*channel\s*=\s*["'](\d+\.\d+(?:\.\d+)?)["']\s*(?:#.*)?$/, 1]
    raise 'Expected a pinned Rust channel in backend/rust-toolchain.toml' unless rust
    dockerfile = File.read(File.join(root, 'deploy/Dockerfile'))
    image = lambda do |stage, name|
      match = dockerfile.match(/^FROM (#{name}:([^\s@]+)@sha256:[a-f0-9]{64}) AS #{stage}$/)
      raise "Missing pinned #{stage} image" unless match
      [match[1], match[2].split('-', 2).first]
    end
    rust_image, image_rust = image.call('backend-builder', 'rust')
    _, node = image.call('frontend-builder', 'node')
    raise "Rust toolchain #{rust} differs from Docker builder #{image_rust}" unless rust == image_rust
    { 'rust' => rust, 'rust_image' => rust_image, 'node' => node }
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    BuildToolchains.read.each { |key, value| puts "#{key}=#{value}" }
  rescue StandardError => error
    warn "error: #{error.message}"
    exit 1
  end
end

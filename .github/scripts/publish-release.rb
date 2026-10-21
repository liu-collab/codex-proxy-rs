require_relative 'release-metadata'

# 构建和验证可以并行；调用方必须在仓库级发布锁内执行本脚本
module ReleasePublication
  def self.run(*command)
    output, error, status = Open3.capture3(*command)
    raise "#{command.first} failed: #{error.strip}" unless status.success?
    output.strip
  end

  def self.latest_release(repository)
    output, error, status = Open3.capture3('gh', 'api', '--include', "repos/#{repository}/releases/latest")
    code = output.lines.first.to_s.split[1]
    return nil if code == '404'
    raise "Cannot read latest GitHub release: #{error.strip}" unless status.success? && code == '200'
    JSON.parse(output.split(/\r?\n\r?\n/, 2).fetch(1)).fetch('tag_name')
  end

  def self.latest_image(image)
    ref = "#{image}:latest"
    output, error, status = Open3.capture3('docker', 'buildx', 'imagetools', 'inspect', ref, '--format', '{{json .Image}}')
    # 只把明确的缺失标签当作首次发布，鉴权和网络错误必须终止提升
    return nil if !status.success? && error.include?("#{ref}: not found")
    raise "Cannot read latest image: #{error.strip}" unless status.success?
    versions = JSON.parse(output).values.map do |config|
      config.fetch('config').fetch('Labels').fetch('org.opencontainers.image.version')
    end.uniq
    raise 'Latest image must have one consistent version across platforms' unless versions.length == 1
    versions.first
  end

  def self.can_promote?(version, current_versions)
    return false if ReleaseMetadata.prerelease?(version)
    current_versions.compact.all? do |current|
      raise "Latest points to a prerelease: #{current}" if ReleaseMetadata.prerelease?(current)
      comparison = ReleaseMetadata.sort_key(version) <=> ReleaseMetadata.sort_key(current)
      # 构建元数据不决定版本先后；同优先级只允许相同版本重试
      comparison.positive? || (comparison.zero? && ReleaseMetadata.version(current) == version)
    end
  end

  def self.tag(image, digest, name)
    ref = "#{image}:#{name}"
    run('docker', 'buildx', 'imagetools', 'create', '-t', ref, "#{image}@#{digest}")
    actual = run('docker', 'buildx', 'imagetools', 'inspect', ref, '--format', '{{.Manifest.Digest}}')
    raise "Promoted image digest changed: #{ref}" unless actual == digest
  end

  def self.publish(version:, image:, digest:, repository:, release_id:)
    version = ReleaseMetadata.version(version)
    raise 'Invalid image digest' unless digest.match?(/\Asha256:[a-f0-9]{64}\z/)
    raise 'Invalid GitHub release ID' unless release_id.match?(/\A\d+\z/)
    endpoint = "repos/#{repository}/releases/#{release_id}"

    # 附件上传及镜像证明、签名已完成，再公开版本入口；失败可从本 job 重跑
    tag(image, digest, version.tr('+', '-'))
    run('gh', 'api', '--method', 'PATCH', endpoint, '-F', 'draft=false', '-f', 'make_latest=false')
    return if ReleaseMetadata.prerelease?(version)

    current_versions = [latest_release(repository), latest_image(image)]
    unless can_promote?(version, current_versions)
      puts "Published #{version}; latest remains at #{current_versions.compact.join(', ')}"
      return
    end

    # 两个服务不能原子更新；同时检查两侧版本，让失败重试可以补齐而旧版不能回退
    tag(image, digest, 'latest')
    run('gh', 'api', '--method', 'PATCH', endpoint, '-f', 'make_latest=true')
    puts "Published #{version} and promoted latest"
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    ReleasePublication.publish(
      version: ENV.fetch('VERSION'), image: "#{ENV.fetch('REGISTRY')}/#{ENV.fetch('IMAGE_NAME')}",
      digest: ENV.fetch('DIGEST'), repository: ENV.fetch('GITHUB_REPOSITORY'),
      release_id: ENV.fetch('RELEASE_ID')
    )
  rescue StandardError => error
    warn "error: #{error.message}"
    exit 1
  end
end

const std = @import("std");
const version_mod = @import("version.zig");
const build_options = @import("build_options");
const log = std.log.scoped(.self_update);

pub const Platform = enum {
    linux_x86_64,
    linux_aarch64,
    macos_x86_64,
    macos_aarch64,
    unknown,

    pub fn current() Platform {
        const os = @import("builtin").os.tag;
        const arch = @import("builtin").cpu.arch;

        return switch (os) {
            .linux => switch (arch) {
                .x86_64 => .linux_x86_64,
                .aarch64 => .linux_aarch64,
                else => .unknown,
            },
            .macos => switch (arch) {
                .x86_64 => .macos_x86_64,
                .aarch64 => .macos_aarch64,
                else => .unknown,
            },
            else => .unknown,
        };
    }

    pub fn assetName(self: Platform) []const u8 {
        return switch (self) {
            .linux_x86_64 => "uppies-linux-x86_64.tar.gz",
            .linux_aarch64 => "uppies-linux-aarch64.tar.gz",
            .macos_x86_64 => "uppies-macos-x86_64.tar.gz",
            .macos_aarch64 => "uppies-macos-aarch64.tar.gz",
            .unknown => "unknown",
        };
    }
};

pub const ReleaseInfo = struct {
    version: []const u8,
    download_url: []const u8,
    allocator: std.mem.Allocator,

    pub fn deinit(self: *ReleaseInfo) void {
        self.allocator.free(self.version);
        self.allocator.free(self.download_url);
    }
};

pub fn getCurrentVersion() []const u8 {
    return build_options.version;
}

pub fn fetchLatestRelease(allocator: std.mem.Allocator, repo: []const u8) !ReleaseInfo {
    const url = try std.fmt.allocPrint(
        allocator,
        "https://api.github.com/repos/{s}/releases/latest",
        .{repo},
    );
    defer allocator.free(url);
    log.debug("Fetching release from: {s}", .{url});

    // Prepare curl command
    const curl_args = [_][]const u8{
        "curl",
        "-sL",
        "-H",
        "Accept: application/vnd.github+json",
        url,
    };

    var child = std.process.Child.init(&curl_args, allocator);
    child.stdout_behavior = .Pipe;
    child.stderr_behavior = .Ignore;

    try child.spawn();
    const stdout = try child.stdout.?.readToEndAlloc(allocator, 10 * 1024 * 1024);
    defer allocator.free(stdout);

    const term = try child.wait();
    if (term != .Exited or term.Exited != 0) {
        return error.FetchFailed;
    }

    log.debug("Received JSON response ({} bytes)", .{stdout.len});
    // Parse JSON response
    return parseReleaseJson(allocator, stdout);
}

fn parseReleaseJson(allocator: std.mem.Allocator, json_text: []const u8) !ReleaseInfo {
    const parsed = try std.json.parseFromSlice(std.json.Value, allocator, json_text, .{});
    defer parsed.deinit();

    const root = parsed.value.object;

    // Get tag_name
    const tag_name_value = root.get("tag_name") orelse return error.InvalidJson;
    const tag_name = tag_name_value.string;
    log.debug("Found tag_name: {s}", .{tag_name});

    // Remove 'v' prefix if present
    const version = if (std.mem.startsWith(u8, tag_name, "v"))
        try allocator.dupe(u8, tag_name[1..])
    else
        try allocator.dupe(u8, tag_name);
    errdefer allocator.free(version);

    // Find the correct asset for current platform
    const platform = Platform.current();
    const asset_name = platform.assetName();
    log.debug("Looking for asset: {s}", .{asset_name});

    // Get assets array
    const assets_value = root.get("assets") orelse return error.InvalidJson;
    const assets = assets_value.array;

    // Find the matching asset
    for (assets.items) |asset_value| {
        const asset = asset_value.object;
        const name_value = asset.get("name") orelse continue;
        const name = name_value.string;

        if (std.mem.eql(u8, name, asset_name)) {
            const url_value = asset.get("browser_download_url") orelse return error.InvalidJson;
            const url = url_value.string;
            const download_url = try allocator.dupe(u8, url);
            log.debug("Found download URL: {s}", .{download_url});

            return ReleaseInfo{
                .version = version,
                .download_url = download_url,
                .allocator = allocator,
            };
        }
    }

    log.debug("Asset not found: {s}", .{asset_name});
    return error.AssetNotFound;
}

pub fn downloadAndExtract(allocator: std.mem.Allocator, url: []const u8, dest_dir: []const u8) !void {
    // Download to temporary file
    const tmp_path = try std.fmt.allocPrint(allocator, "{s}/uppies-download.tar.gz", .{dest_dir});
    defer allocator.free(tmp_path);

    // Download with curl
    const curl_args = [_][]const u8{
        "curl",
        "-sL",
        "-o",
        tmp_path,
        url,
    };

    var curl_child = std.process.Child.init(&curl_args, allocator);
    curl_child.stdout_behavior = .Ignore;
    curl_child.stderr_behavior = .Inherit;

    try curl_child.spawn();
    const curl_term = try curl_child.wait();
    if (curl_term != .Exited or curl_term.Exited != 0) {
        return error.DownloadFailed;
    }

    // Extract with tar
    const tar_args = [_][]const u8{
        "tar",
        "-xzf",
        tmp_path,
        "-C",
        dest_dir,
    };

    var tar_child = std.process.Child.init(&tar_args, allocator);
    tar_child.stdout_behavior = .Ignore;
    tar_child.stderr_behavior = .Inherit;

    try tar_child.spawn();
    const tar_term = try tar_child.wait();

    // Clean up downloaded archive
    std.fs.cwd().deleteFile(tmp_path) catch {};

    if (tar_term != .Exited or tar_term.Exited != 0) {
        return error.ExtractFailed;
    }
}

pub fn replaceBinary(allocator: std.mem.Allocator, new_binary_path: []const u8, current_binary_path: []const u8) !void {
    // Make new binary executable
    const chmod_args = [_][]const u8{
        "chmod",
        "+x",
        new_binary_path,
    };

    var chmod_child = std.process.Child.init(&chmod_args, allocator);
    try chmod_child.spawn();
    _ = try chmod_child.wait();

    // Create backup of current binary
    const backup_path = try std.fmt.allocPrint(allocator, "{s}.backup", .{current_binary_path});
    defer allocator.free(backup_path);

    std.fs.cwd().deleteFile(backup_path) catch {};
    try std.fs.cwd().copyFile(current_binary_path, std.fs.cwd(), backup_path, .{});

    // Replace current binary with new one
    try std.fs.cwd().deleteFile(current_binary_path);
    try std.fs.cwd().rename(new_binary_path, current_binary_path);
}

test "platform detection" {
    const platform = Platform.current();
    try std.testing.expect(platform != .unknown);

    const asset = platform.assetName();
    try std.testing.expect(asset.len > 0);
}

test "version parsing" {
    const allocator = std.testing.allocator;

    const json =
        \\{
        \\  "tag_name": "v1.2.3",
        \\  "name": "Release 1.2.3",
        \\  "assets": [
        \\    {
        \\      "name": "uppies-linux-x86_64.tar.gz",
        \\      "browser_download_url": "https://example.com/download.tar.gz"
        \\    }
        \\  ]
        \\}
    ;

    var info = try parseReleaseJson(allocator, json);
    defer info.deinit();

    try std.testing.expectEqualStrings("1.2.3", info.version);
    try std.testing.expectEqualStrings("https://example.com/download.tar.gz", info.download_url);
}

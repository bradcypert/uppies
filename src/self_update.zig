const std = @import("std");
const version_mod = @import("version.zig");
const build_options = @import("build_options");

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

    // Parse JSON response
    return parseReleaseJson(allocator, stdout);
}

fn parseReleaseJson(allocator: std.mem.Allocator, json: []const u8) !ReleaseInfo {
    // Simple JSON parsing - look for tag_name and asset download_url
    const tag_name = try findJsonString(allocator, json, "tag_name");
    errdefer allocator.free(tag_name);

    // Remove 'v' prefix if present
    const version = if (std.mem.startsWith(u8, tag_name, "v"))
        try allocator.dupe(u8, tag_name[1..])
    else
        try allocator.dupe(u8, tag_name);
    allocator.free(tag_name);
    errdefer allocator.free(version);

    // Find the correct asset for current platform
    const platform = Platform.current();
    const asset_name = platform.assetName();

    const download_url = try findAssetUrl(allocator, json, asset_name);

    return ReleaseInfo{
        .version = version,
        .download_url = download_url,
        .allocator = allocator,
    };
}

fn findJsonString(allocator: std.mem.Allocator, json: []const u8, key: []const u8) ![]const u8 {
    const search = try std.fmt.allocPrint(allocator, "\"{s}\":", .{key});
    defer allocator.free(search);

    const pos = std.mem.indexOf(u8, json, search) orelse return error.KeyNotFound;
    const after_key = json[pos + search.len ..];

    // Skip whitespace and find opening quote
    var i: usize = 0;
    while (i < after_key.len and after_key[i] != '"') : (i += 1) {}
    if (i >= after_key.len) return error.InvalidJson;

    const start = i + 1;
    const end = std.mem.indexOfPos(u8, after_key, start, "\"") orelse return error.InvalidJson;

    return try allocator.dupe(u8, after_key[start..end]);
}

fn findAssetUrl(allocator: std.mem.Allocator, json: []const u8, asset_name: []const u8) ![]const u8 {
    // Find the asset with matching name
    const name_search = try std.fmt.allocPrint(allocator, "\"name\":\"{s}\"", .{asset_name});
    defer allocator.free(name_search);

    var search_start: usize = 0;
    while (std.mem.indexOfPos(u8, json, search_start, name_search)) |pos| {
        // Found the asset, now find its browser_download_url
        // Look backwards for the start of this asset object
        const asset_start = std.mem.lastIndexOfScalar(u8, json[0..pos], '{') orelse pos;
        // Look forwards for the end of this asset object
        const asset_end = std.mem.indexOfPos(u8, json, pos, "}") orelse json.len;
        const asset_json = json[asset_start..asset_end];

        // Find browser_download_url in this asset
        if (findJsonString(allocator, asset_json, "browser_download_url")) |url| {
            return url;
        } else |_| {}

        search_start = pos + 1;
    }

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
        \\  "name": "Release 1.2.3"
        \\}
    ;
    
    const tag = try findJsonString(allocator, json, "tag_name");
    defer allocator.free(tag);
    
    try std.testing.expectEqualStrings("v1.2.3", tag);
}

const std = @import("std");
const uppies = @import("uppies");
const Config = @import("config.zig").Config;
const version_mod = @import("version.zig");
const self_update = @import("self_update.zig");

const Command = enum {
    list,
    check,
    update,
    @"self-update",
    version,
    help,
};

const Args = struct {
    command: Command,
    app_name: ?[]const u8 = null,
    force: bool = false,
};

fn parseArgs(allocator: std.mem.Allocator) !Args {
    var args = try std.process.argsWithAllocator(allocator);
    defer args.deinit();

    _ = args.skip();

    const first = args.next() orelse return Args{ .command = .help };

    if (std.mem.eql(u8, first, "list")) {
        return Args{ .command = .list };
    } else if (std.mem.eql(u8, first, "check")) {
        return Args{ .command = .check };
    } else if (std.mem.eql(u8, first, "self-update")) {
        return Args{ .command = .@"self-update" };
    } else if (std.mem.eql(u8, first, "version") or std.mem.eql(u8, first, "--version") or std.mem.eql(u8, first, "-v")) {
        return Args{ .command = .version };
    } else if (std.mem.eql(u8, first, "update")) {
        var result = Args{ .command = .update };
        while (args.next()) |arg| {
            if (std.mem.eql(u8, arg, "--force")) {
                result.force = true;
            } else {
                result.app_name = arg;
            }
        }
        return result;
    } else {
        return Args{ .command = .help };
    }
}

fn printHelp() void {
    const help =
        \\uppies - app update orchestrator
        \\
        \\USAGE:
        \\    uppies <command> [options]
        \\
        \\COMMANDS:
        \\    list              List all registered apps
        \\    check             Check local vs remote versions
        \\    update [app]      Update app(s) if versions differ
        \\    self-update       Update uppies itself
        \\    version           Show version information
        \\
        \\OPTIONS:
        \\    --force           Bypass version check
        \\
    ;
    std.debug.print("{s}\n", .{help});
}

fn printVersion() void {
    const version = self_update.getCurrentVersion();
    std.debug.print("uppies version {s}\n", .{version});
}

fn getConfigPath(allocator: std.mem.Allocator) ![]const u8 {
    const home = std.posix.getenv("HOME") orelse return error.NoHomeDir;
    return try std.fmt.allocPrint(allocator, "{s}/.local/share/uppies/apps.toml", .{home});
}

fn cmdList(_: std.mem.Allocator, config: *Config) !void {
    var stdout_buf: [4096]u8 = undefined;
    var stdout_writer = std.fs.File.stdout().writer(&stdout_buf);
    const stdout = &stdout_writer.interface;

    if (config.apps.len == 0) {
        try stdout.print("No apps registered\n", .{});
        try stdout.flush();
        return;
    }

    for (config.apps) |app| {
        if (app.description) |desc| {
            try stdout.print("{s: <20} {s}\n", .{ app.name, desc });
        } else {
            try stdout.print("{s}\n", .{app.name});
        }
    }
    try stdout.flush();
}

fn cmdCheck(allocator: std.mem.Allocator, config: *Config) !void {
    var stdout_buf: [4096]u8 = undefined;
    var stderr_buf: [4096]u8 = undefined;
    var stdout_writer = std.fs.File.stdout().writer(&stdout_buf);
    var stderr_writer = std.fs.File.stderr().writer(&stderr_buf);
    const stdout = &stdout_writer.interface;
    const stderr = &stderr_writer.interface;

    for (config.apps) |app| {
        var local_result = uppies.runScript(allocator, app.local.script) catch |err| {
            try stderr.print("{s}: local version script failed ({any})\n", .{ app.name, err });
            continue;
        };
        defer local_result.deinit();

        if (local_result.exit_code != 0) {
            try stderr.print("{s}: local version script failed (exit {d})\n", .{ app.name, local_result.exit_code });
            continue;
        }

        var remote_result = uppies.runScript(allocator, app.remote.script) catch |err| {
            try stderr.print("{s}: remote version script failed ({any})\n", .{ app.name, err });
            continue;
        };
        defer remote_result.deinit();

        if (remote_result.exit_code != 0) {
            try stderr.print("{s}: remote version script failed (exit {d})\n", .{ app.name, remote_result.exit_code });
            continue;
        }

        const local_ver = uppies.trimVersion(local_result.stdout);
        const remote_ver = uppies.trimVersion(remote_result.stdout);

        const needs_update = switch (app.compare_mode) {
            .string => !std.mem.eql(u8, local_ver, remote_ver),
            .semver => blk: {
                const local_semver = version_mod.Version.parse(local_ver) catch {
                    try stderr.print("{s}: failed to parse local version as semver: {s}\n", .{ app.name, local_ver });
                    continue;
                };
                const remote_semver = version_mod.Version.parse(remote_ver) catch {
                    try stderr.print("{s}: failed to parse remote version as semver: {s}\n", .{ app.name, remote_ver });
                    continue;
                };
                break :blk local_semver.lessThan(remote_semver);
            },
        };

        if (needs_update) {
            try stdout.print("{s: <20} {s: <15} → {s: <15} (update available)\n", .{ app.name, local_ver, remote_ver });
        } else {
            try stdout.print("{s: <20} {s: <15} (up to date)\n", .{ app.name, local_ver });
        }
    }
    try stdout.flush();
    try stderr.flush();
}

fn cmdUpdate(allocator: std.mem.Allocator, config: *Config, app_name: ?[]const u8, force: bool) !void {
    var stdout_buf: [4096]u8 = undefined;
    var stderr_buf: [4096]u8 = undefined;
    var stdout_writer = std.fs.File.stdout().writer(&stdout_buf);
    var stderr_writer = std.fs.File.stderr().writer(&stderr_buf);
    const stdout = &stdout_writer.interface;
    const stderr = &stderr_writer.interface;

    for (config.apps) |app| {
        if (app_name) |name| {
            if (!std.mem.eql(u8, app.name, name)) continue;
        }

        var should_update = force;

        if (!force) {
            var local_result = uppies.runScript(allocator, app.local.script) catch |err| {
                try stderr.print("{s}: local version script failed ({any})\n", .{ app.name, err });
                continue;
            };
            defer local_result.deinit();

            if (local_result.exit_code != 0) {
                try stderr.print("{s}: local version script failed (exit {d})\n", .{ app.name, local_result.exit_code });
                continue;
            }

            var remote_result = uppies.runScript(allocator, app.remote.script) catch |err| {
                try stderr.print("{s}: remote version script failed ({any})\n", .{ app.name, err });
                continue;
            };
            defer remote_result.deinit();

            if (remote_result.exit_code != 0) {
                try stderr.print("{s}: remote version script failed (exit {d})\n", .{ app.name, remote_result.exit_code });
                continue;
            }

            const local_ver = uppies.trimVersion(local_result.stdout);
            const remote_ver = uppies.trimVersion(remote_result.stdout);

            const needs_update = switch (app.compare_mode) {
                .string => !std.mem.eql(u8, local_ver, remote_ver),
                .semver => blk: {
                    const local_semver = version_mod.Version.parse(local_ver) catch {
                        try stderr.print("{s}: failed to parse local version as semver: {s}\n", .{ app.name, local_ver });
                        continue;
                    };
                    const remote_semver = version_mod.Version.parse(remote_ver) catch {
                        try stderr.print("{s}: failed to parse remote version as semver: {s}\n", .{ app.name, remote_ver });
                        continue;
                    };
                    break :blk local_semver.lessThan(remote_semver);
                },
            };

            if (needs_update) {
                should_update = true;
                try stdout.print("{s}: updating {s} → {s}\n", .{ app.name, local_ver, remote_ver });
            } else {
                try stdout.print("{s}: already up to date ({s})\n", .{ app.name, local_ver });
            }
        }

        if (should_update) {
            try stdout.print("{s}: running update script...\n", .{app.name});

            var update_result = uppies.runScript(allocator, app.update.script) catch |err| {
                try stderr.print("{s}: update script failed ({any})\n", .{ app.name, err });
                continue;
            };
            defer update_result.deinit();

            if (update_result.exit_code != 0) {
                try stderr.print("{s}: update script failed (exit {d})\n", .{ app.name, update_result.exit_code });
                continue;
            }

            try stdout.print("{s}: update complete\n", .{app.name});
        }
    }
    try stdout.flush();
    try stderr.flush();
}

fn cmdSelfUpdate(allocator: std.mem.Allocator, repo: []const u8) !void {
    var stdout_buf: [4096]u8 = undefined;
    var stderr_buf: [4096]u8 = undefined;
    var stdout_writer = std.fs.File.stdout().writer(&stdout_buf);
    var stderr_writer = std.fs.File.stderr().writer(&stderr_buf);
    const stdout = &stdout_writer.interface;
    const stderr = &stderr_writer.interface;

    const platform = self_update.Platform.current();
    if (platform == .unknown) {
        try stderr.print("Error: Unsupported platform for self-update\n", .{});
        try stderr.flush();
        return error.UnsupportedPlatform;
    }

    try stdout.print("Checking for updates...\n", .{});
    try stdout.flush();

    // Fetch latest release info
    var release_info = self_update.fetchLatestRelease(allocator, repo) catch |err| {
        try stderr.print("Failed to fetch latest release: {any}\n", .{err});
        try stderr.print("Make sure you have internet connection and curl installed\n", .{});
        try stderr.flush();
        return err;
    };
    defer release_info.deinit();

    const current_version = self_update.getCurrentVersion();

    try stdout.print("Current version: {s}\n", .{current_version});
    try stdout.print("Latest version:  {s}\n", .{release_info.version});
    try stdout.flush();

    // Compare versions
    const current_semver = version_mod.Version.parse(current_version) catch {
        try stderr.print("Failed to parse current version as semver\n", .{});
        try stderr.flush();
        return error.InvalidVersion;
    };

    const latest_semver = version_mod.Version.parse(release_info.version) catch {
        try stderr.print("Failed to parse latest version as semver\n", .{});
        try stderr.flush();
        return error.InvalidVersion;
    };

    if (current_semver.equals(latest_semver)) {
        try stdout.print("Already up to date!\n", .{});
        try stdout.flush();
        return;
    }

    if (!current_semver.lessThan(latest_semver)) {
        try stdout.print("Current version is newer than latest release\n", .{});
        try stdout.flush();
        return;
    }

    try stdout.print("\nDownloading uppies {s}...\n", .{release_info.version});
    try stdout.flush();

    // Get current executable path
    const exe_path = try std.fs.selfExePathAlloc(allocator);
    defer allocator.free(exe_path);

    // Create temporary directory
    const tmp_dir = try std.fmt.allocPrint(allocator, "/tmp/uppies-update-{d}", .{std.time.timestamp()});
    defer allocator.free(tmp_dir);

    std.fs.cwd().makeDir(tmp_dir) catch |err| {
        try stderr.print("Failed to create temp directory: {any}\n", .{err});
        try stderr.flush();
        return err;
    };
    defer std.fs.cwd().deleteTree(tmp_dir) catch {};

    // Download and extract
    try self_update.downloadAndExtract(allocator, release_info.download_url, tmp_dir);

    // Get path to new binary
    const new_binary = try std.fmt.allocPrint(allocator, "{s}/uppies", .{tmp_dir});
    defer allocator.free(new_binary);

    try stdout.print("Installing...\n", .{});
    try stdout.flush();

    // Replace binary
    self_update.replaceBinary(allocator, new_binary, exe_path) catch |err| {
        try stderr.print("Failed to replace binary: {any}\n", .{err});
        try stderr.print("You may need to run with sudo or install to a writable location\n", .{});
        try stderr.flush();
        return err;
    };

    try stdout.print("\n✓ Successfully updated to version {s}!\n", .{release_info.version});
    try stdout.flush();
}

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const args = try parseArgs(allocator);

    if (args.command == .help) {
        printHelp();
        return;
    }

    if (args.command == .version) {
        printVersion();
        return;
    }

    if (args.command == .@"self-update") {
        // Use GitHub repo from environment or default
        // Set via: export UPPIES_REPO=username/uppies
        const repo = std.posix.getenv("UPPIES_REPO") orelse "bradcypert/uppies";
        try cmdSelfUpdate(allocator, repo);
        return;
    }

    const config_path = try getConfigPath(allocator);
    defer allocator.free(config_path);

    var config = Config.loadFromFile(allocator, config_path) catch |err| {
        var stderr_buf: [4096]u8 = undefined;
        var stderr_writer = std.fs.File.stderr().writer(&stderr_buf);
        const stderr = &stderr_writer.interface;

        const err_msg = switch (err) {
            error.FileNotFound => "Config file not found",
            error.InvalidToml => "Invalid TOML syntax",
            error.AccessDenied => "Permission denied",
            else => "Failed to load config",
        };

        try stderr.print("{s}: {s}\n", .{ err_msg, config_path });
        try stderr.print("Expected format: ~/.local/share/uppies/apps.toml\n", .{});
        try stderr.print("See example/apps.toml for reference\n", .{});
        try stderr.flush();
        return err;
    };
    defer config.deinit();

    config.validate() catch |err| {
        var stderr_buf: [4096]u8 = undefined;
        var stderr_writer = std.fs.File.stderr().writer(&stderr_buf);
        const stderr = &stderr_writer.interface;

        const err_msg = switch (err) {
            error.FileNotFound => "Script file not found",
            error.NotAFile => "Script path is not a file",
            error.NotExecutable => "Script is not executable (chmod +x)",
            error.InvalidConfig => "Invalid configuration",
            else => "Configuration validation failed",
        };

        try stderr.print("Config validation error: {s}\n", .{err_msg});
        try stderr.flush();
        return err;
    };

    switch (args.command) {
        .list => try cmdList(allocator, &config),
        .check => try cmdCheck(allocator, &config),
        .update => try cmdUpdate(allocator, &config, args.app_name, args.force),
        .@"self-update", .version => unreachable, // Handled above
        .help => printHelp(),
    }
}

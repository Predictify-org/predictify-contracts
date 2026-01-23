# How to View Errors from Your Phone on Your PC

## Method 1: Metro Bundler Terminal (Easiest) ⭐

**Errors automatically appear in your terminal where you ran `npm start`**

1. Look at the terminal where Metro bundler is running
2. All errors, warnings, and console.logs from your phone will appear there
3. Errors are shown in **RED**
4. Warnings are shown in **YELLOW**

**Example:**
```
ERROR  Warning: Cannot read property 'map' of undefined
ERROR  TypeError: undefined is not an object (evaluating 'lessons.map')
```

## Method 2: Expo Dev Tools (Browser)

1. When Metro bundler starts, you'll see a QR code
2. Press **`j`** in the terminal to open the debugger
3. OR open this URL in your browser: `http://localhost:19002`
4. You'll see:
   - Logs
   - Errors
   - Network requests
   - Performance metrics

## Method 3: React Native Debugger

1. Install React Native Debugger (optional):
   ```bash
   # Download from: https://github.com/jhen0409/react-native-debugger/releases
   ```

2. Enable Remote JS Debugging:
   - Shake your phone (or press `Cmd+D` on iOS simulator / `Cmd+M` on Android)
   - Select "Debug" or "Open Developer Menu"
   - Tap "Debug" or "Debug with Chrome"

3. Open Chrome DevTools:
   - Go to `chrome://inspect`
   - Click "inspect" under your device

## Method 4: Check Error Boundary Screen

The app now has an Error Boundary that will:
- Catch errors and display them on screen
- Show error details
- Log errors to console (visible in Metro terminal)

## Method 5: Enable Verbose Logging

Add this to see more details:

```typescript
// In your component
console.log('Debug info:', yourData);
console.error('Error details:', error);
```

All console.log/error statements appear in the Metro terminal!

## Quick Tips:

✅ **Always check your Metro terminal first** - it shows everything
✅ **Errors appear in RED** in the terminal
✅ **Press `r` in terminal** to reload and see new errors
✅ **Press `j` in terminal** to open debugger in browser
✅ **Shake your phone** to open developer menu

## Common Commands:

- `r` - Reload app
- `j` - Open debugger
- `m` - Toggle menu
- `Ctrl+C` - Stop Metro bundler

---

**The easiest way: Just look at your terminal where `npm start` is running! All errors appear there automatically.** 🎯

//% MAIN_ACTIVITY_BODY
private int padId = -1;
private int padButtons = 0;
private float padX = 0, padY = 0;
private native void nativePadState(int connected, int buttons, float x, float y);
private native void nativeBack();

public void quitGame() { runOnUiThread(new Runnable() { public void run() {
    finish();
    // Miniquad keeps process-global native state after its render loop ends.
    // End this app's process after Android has returned to the launcher so a
    // subsequent launch always initializes a fresh native runtime.
    new android.os.Handler(android.os.Looper.getMainLooper()).postDelayed(new Runnable() {
        public void run() { android.os.Process.killProcess(android.os.Process.myPid()); }
    }, 250);
} }); }

public void openSupport() {
    runOnUiThread(new Runnable() { public void run() {
        try { startActivity(new android.content.Intent(android.content.Intent.ACTION_VIEW,
            android.net.Uri.parse("https://buymeacoffee.com/adamstrad"))); }
        catch (android.content.ActivityNotFoundException ignored) { }
    }});
}

private void enterImmersive() {
    if (android.os.Build.VERSION.SDK_INT >= 30) {
        android.view.WindowInsetsController controller = getWindow().getInsetsController();
        if (controller != null) {
            controller.setSystemBarsBehavior(android.view.WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
            controller.hide(android.view.WindowInsets.Type.systemBars());
        }
    } else {
        getWindow().getDecorView().setSystemUiVisibility(
            android.view.View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | android.view.View.SYSTEM_UI_FLAG_FULLSCREEN |
            android.view.View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY);
    }
}

private void refreshPads() {
    int chosen = -1;
    for (int id : android.view.InputDevice.getDeviceIds()) {
        android.view.InputDevice device = android.view.InputDevice.getDevice(id);
        if (device != null && !device.isVirtual() &&
            ((device.getSources() & android.view.InputDevice.SOURCE_GAMEPAD) == android.view.InputDevice.SOURCE_GAMEPAD ||
             ((device.getSources() & android.view.InputDevice.SOURCE_JOYSTICK) == android.view.InputDevice.SOURCE_JOYSTICK &&
              device.getMotionRange(android.view.MotionEvent.AXIS_X) != null &&
              device.getMotionRange(android.view.MotionEvent.AXIS_Y) != null))) {
            chosen = id;
            if (id == padId) break;
        }
    }
    if (chosen != padId) { padId = chosen; padButtons = 0; padX = padY = 0; }
    nativePadState(padId < 0 ? 0 : 1, padButtons, padX, padY);
}

@Override public boolean dispatchKeyEvent(android.view.KeyEvent event) {
    int key = event.getKeyCode();
    if (key == android.view.KeyEvent.KEYCODE_BACK) {
        if (event.getAction() == android.view.KeyEvent.ACTION_UP) nativeBack();
        return true;
    }
    if (key == android.view.KeyEvent.KEYCODE_VOLUME_UP || key == android.view.KeyEvent.KEYCODE_VOLUME_DOWN ||
        key == android.view.KeyEvent.KEYCODE_VOLUME_MUTE) {
        if (event.getAction() == android.view.KeyEvent.ACTION_DOWN) {
            android.media.AudioManager audio = (android.media.AudioManager)getSystemService(AUDIO_SERVICE);
            int direction = key == android.view.KeyEvent.KEYCODE_VOLUME_UP ? android.media.AudioManager.ADJUST_RAISE :
                key == android.view.KeyEvent.KEYCODE_VOLUME_DOWN ? android.media.AudioManager.ADJUST_LOWER :
                android.media.AudioManager.ADJUST_TOGGLE_MUTE;
            audio.adjustStreamVolume(android.media.AudioManager.STREAM_MUSIC, direction, android.media.AudioManager.FLAG_SHOW_UI);
        }
        return true;
    }
    if (event.getDeviceId() == padId && padId >= 0) {
        int bit = 0;
        switch (event.getKeyCode()) {
            case android.view.KeyEvent.KEYCODE_BUTTON_A: bit = 1; break;
            case android.view.KeyEvent.KEYCODE_DPAD_LEFT: bit = 2; break;
            case android.view.KeyEvent.KEYCODE_DPAD_RIGHT: bit = 4; break;
            case android.view.KeyEvent.KEYCODE_DPAD_UP: bit = 8; break;
            case android.view.KeyEvent.KEYCODE_DPAD_DOWN: bit = 16; break;
        }
        if (bit != 0) {
            if (event.getAction() == android.view.KeyEvent.ACTION_DOWN) padButtons |= bit;
            else if (event.getAction() == android.view.KeyEvent.ACTION_UP) padButtons &= ~bit;
            nativePadState(1, padButtons, padX, padY);
            return true;
        }
    }
    return super.dispatchKeyEvent(event);
}

@Override public boolean dispatchGenericMotionEvent(android.view.MotionEvent event) {
    if (event.getDeviceId() == padId && padId >= 0 &&
        (event.getSource() & android.view.InputDevice.SOURCE_JOYSTICK) == android.view.InputDevice.SOURCE_JOYSTICK) {
        padX = event.getAxisValue(android.view.MotionEvent.AXIS_X);
        padY = event.getAxisValue(android.view.MotionEvent.AXIS_Y);
        float hx = event.getAxisValue(android.view.MotionEvent.AXIS_HAT_X);
        float hy = event.getAxisValue(android.view.MotionEvent.AXIS_HAT_Y);
        if (Math.abs(hx) > Math.abs(padX)) padX = hx;
        if (Math.abs(hy) > Math.abs(padY)) padY = hy;
        nativePadState(1, padButtons, padX, padY);
        return true;
    }
    return super.dispatchGenericMotionEvent(event);
}
//% END

//% MAIN_ACTIVITY_ON_CREATE
android.hardware.input.InputManager manager = (android.hardware.input.InputManager)getSystemService(INPUT_SERVICE);
manager.registerInputDeviceListener(new android.hardware.input.InputManager.InputDeviceListener() {
    public void onInputDeviceAdded(int id) { refreshPads(); }
    public void onInputDeviceRemoved(int id) { refreshPads(); }
    public void onInputDeviceChanged(int id) { refreshPads(); }
}, null);
refreshPads();
enterImmersive();
if (android.os.Build.VERSION.SDK_INT >= 33) {
    getOnBackInvokedDispatcher().registerOnBackInvokedCallback(
        android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT,
        new android.window.OnBackInvokedCallback() { public void onBackInvoked() { nativeBack(); } });
}
//% END

//% MAIN_ACTIVITY_ON_RESUME
enterImmersive();
//% END

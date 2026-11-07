package com.tauri.dev.music

import android.app.*
import android.content.*
import android.database.Cursor
import android.graphics.*
import android.media.*
import android.media.session.MediaSession
import android.media.session.PlaybackState
import android.net.Uri
import android.os.*
import org.json.JSONObject
import java.util.UUID
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit

/** Initializes Rust for both the Activity and cold AppFunction service starts. */
class MusicProvider : ContentProvider() {
    private external fun initialize(context: Context)
    override fun onCreate(): Boolean {
        System.loadLibrary("tauri_app_lib")
        initialize(context!!.applicationContext)
        return true
    }
    override fun query(u: Uri, p: Array<out String>?, s: String?, a: Array<out String>?, o: String?): Cursor? = null
    override fun getType(u: Uri): String? = null
    override fun insert(u: Uri, v: ContentValues?): Uri? = null
    override fun delete(u: Uri, s: String?, a: Array<out String>?): Int = 0
    override fun update(u: Uri, v: ContentValues?, s: String?, a: Array<out String>?): Int = 0
}

/** The UI, AppFunctions, notification and lock screen all control this one player. */
class MusicService : Service() {
    private var player: MediaPlayer? = null
    private var index = 0
    private var volume = 70
    private lateinit var session: MediaSession
    private lateinit var audio: AudioManager
    private lateinit var focus: AudioFocusRequest
    private var resumeOnFocus = false
    private val artwork by lazy { arrayOf(makeArtwork(0), makeArtwork(1)) }
    private val attributes = AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA)
        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build()

    override fun onCreate() {
        super.onCreate()
        instance = this
        val prefs = getSharedPreferences("music", MODE_PRIVATE)
        index = prefs.getInt("track", 0).coerceIn(0, titles.lastIndex)
        volume = prefs.getInt("volume", 70)
        audio = getSystemService(AudioManager::class.java)
        focus = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN).setAudioAttributes(attributes)
            .setOnAudioFocusChangeListener({ change ->
                when (change) {
                    AudioManager.AUDIOFOCUS_LOSS -> { resumeOnFocus = false; perform("pause") }
                    AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> {
                        resumeOnFocus = player?.isPlaying == true
                        player?.pause(); publish()
                    }
                    AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK -> player?.setVolume(.15f, .15f)
                    AudioManager.AUDIOFOCUS_GAIN -> {
                        applyVolume()
                        if (resumeOnFocus) { player?.start(); resumeOnFocus = false; publish() }
                    }
                }
            }, Handler(mainLooper)).build()
        session = MediaSession(this, "Small Hours").apply {
            setCallback(object : MediaSession.Callback() {
                override fun onPlay() { perform("play") }
                override fun onPause() { perform("pause") }
                override fun onSkipToNext() { perform("next") }
                override fun onSkipToPrevious() { perform("previous") }
                override fun onStop() { perform("pause"); stopSelf() }
                override fun onSeekTo(pos: Long) { perform("seek:$pos") }
            })
            isActive = true
        }
        getSystemService(NotificationManager::class.java).createNotificationChannel(
            NotificationChannel("music", "Music", NotificationManager.IMPORTANCE_LOW)
        )
        updateSession()
        startForeground(10, notification())
    }

    private fun action(name: String, icon: Int, label: String): Notification.Action {
        val intent = Intent(this, MusicService::class.java).setAction(name)
        val pending = PendingIntent.getService(this, name.hashCode(), intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        return Notification.Action.Builder(icon, label, pending).build()
    }

    private fun notification(): Notification {
        val playing = player?.isPlaying == true
        return Notification.Builder(this, "music")
            .setSmallIcon(android.R.drawable.ic_media_play)
            .setContentTitle(titles[index]).setContentText("Small Hours · Demo sessions")
            .setLargeIcon(artwork[index]).setVisibility(Notification.VISIBILITY_PUBLIC)
            .setOnlyAlertOnce(true).setShowWhen(false).setOngoing(playing)
            .addAction(action("previous", android.R.drawable.ic_media_previous, "Previous"))
            .addAction(action(if (playing) "pause" else "play",
                if (playing) android.R.drawable.ic_media_pause else android.R.drawable.ic_media_play,
                if (playing) "Pause" else "Play"))
            .addAction(action("next", android.R.drawable.ic_media_next, "Next"))
            .setStyle(Notification.MediaStyle().setMediaSession(session.sessionToken)
                .setShowActionsInCompactView(0, 1, 2))
            .build()
    }

    private fun updateSession() {
        session.setMetadata(MediaMetadata.Builder()
            .putString(MediaMetadata.METADATA_KEY_TITLE, titles[index])
            .putString(MediaMetadata.METADATA_KEY_ARTIST, "Small Hours")
            .putString(MediaMetadata.METADATA_KEY_ALBUM, "Demo sessions")
            .putLong(MediaMetadata.METADATA_KEY_DURATION, 16000)
            .putBitmap(MediaMetadata.METADATA_KEY_ALBUM_ART, artwork[index]).build())
        val playing = player?.isPlaying == true
        session.setPlaybackState(PlaybackState.Builder()
            .setActions(PlaybackState.ACTION_PLAY or PlaybackState.ACTION_PAUSE or
                PlaybackState.ACTION_PLAY_PAUSE or PlaybackState.ACTION_SKIP_TO_NEXT or
                PlaybackState.ACTION_SKIP_TO_PREVIOUS or PlaybackState.ACTION_SEEK_TO or PlaybackState.ACTION_STOP)
            .setState(if (playing) PlaybackState.STATE_PLAYING else PlaybackState.STATE_PAUSED,
                player?.currentPosition?.toLong() ?: 0, if (playing) 1f else 0f)
            .build())
    }

    private fun applyVolume() { val gain = volume / 100f; player?.setVolume(gain, gain) }

    private fun startTrack(target: Int) {
        startForeground(10, notification())
        check(audio.requestAudioFocus(focus) == AudioManager.AUDIOFOCUS_REQUEST_GRANTED) { "Audio focus unavailable" }
        if (target != index || player == null) {
            player?.release()
            player = null
            index = target
            assets.openFd("music/${files[index]}").use { descriptor ->
                player = MediaPlayer().apply {
                    setAudioAttributes(attributes)
                    setDataSource(descriptor.fileDescriptor, descriptor.startOffset, descriptor.length)
                    isLooping = true
                    prepare()
                }
            }
        }
        applyVolume()
        player!!.start()
    }

    private fun perform(command: String): String {
        when {
            command == "play" -> startTrack(index)
            command == "next" -> startTrack((index + 1) % titles.size)
            command == "previous" -> startTrack((index + titles.size - 1) % titles.size)
            command.startsWith("track:") -> startTrack(command.substringAfter(':').toInt().also { require(it in titles.indices) })
            command == "pause" -> {
                resumeOnFocus = false
                player?.pause()
                audio.abandonAudioFocusRequest(focus)
            }
            command.startsWith("volume:") -> { volume = command.substringAfter(':').toInt().coerceIn(0, 100); applyVolume() }
            command.startsWith("seek:") -> player?.seekTo(command.substringAfter(':').toInt().coerceIn(0, 15999))
            command == "state" -> return currentState()
            else -> error("Unknown music command")
        }
        getSharedPreferences("music", MODE_PRIVATE).edit().putInt("track", index).putInt("volume", volume).apply()
        publish()
        if (command == "pause") {
            // Keep the paused media card and session available for Resume, without a foreground service.
            stopForeground(STOP_FOREGROUND_DETACH)
        }
        return snapshot!!
    }

    private fun currentState(): String = state(index, player?.isPlaying == true, volume, player?.currentPosition ?: 0)
    private fun publish() {
        updateSession()
        snapshot = currentState()
        getSystemService(NotificationManager::class.java).notify(10, notification())
        playbackChanged(snapshot!!)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val request = intent?.getStringExtra("request")?.let { pending.remove(it) }
        try {
            val result = perform(intent?.action ?: "state")
            request?.complete(result)
        }
        catch (error: Exception) {
            request?.completeExceptionally(error)
            android.util.Log.e("SmallHours", "Playback command failed", error)
            if (player == null) stopSelf()
        }
        return START_NOT_STICKY
    }
    override fun onDestroy() {
        player?.release(); player = null
        audio.abandonAudioFocusRequest(focus)
        snapshot = state(index, false, volume, 0)
        playbackChanged(snapshot!!)
        session.release()
        instance = null
        getSystemService(NotificationManager::class.java).cancel(10)
        super.onDestroy()
    }
    override fun onBind(intent: Intent?): IBinder? = null

    private fun makeArtwork(track: Int): Bitmap {
        val bitmap = Bitmap.createBitmap(512, 512, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bitmap)
        val paint = Paint(Paint.ANTI_ALIAS_FLAG)
        val colors = if (track == 0) intArrayOf(0xffcbd5b7.toInt(), 0xfffaf3d7.toInt())
            else intArrayOf(0xff132c42.toInt(), 0xffaf6d5b.toInt())
        paint.shader = LinearGradient(0f, 0f, 512f, 512f, colors, null, Shader.TileMode.CLAMP)
        canvas.drawRect(0f, 0f, 512f, 512f, paint)
        paint.shader = null; paint.color = if (track == 0) 0xfff1b372.toInt() else 0xffe6ceaa.toInt()
        canvas.drawCircle(if (track == 0) 300f else 355f, 200f, if (track == 0) 96f else 64f, paint)
        paint.color = if (track == 0) 0xff4d6c60.toInt() else 0xff182939.toInt()
        canvas.drawOval(-160f, 305f, 680f, 760f, paint)
        return bitmap
    }

    companion object {
        private val titles = arrayOf("Morning Loop", "Night Drive")
        private val files = arrayOf("morning.wav", "night.wav")
        private val pending = ConcurrentHashMap<String, CompletableFuture<String>>()
        @Volatile private var instance: MusicService? = null
        @Volatile private var snapshot: String? = null
        @JvmStatic private external fun playbackChanged(json: String)
        private fun state(index: Int, playing: Boolean, volume: Int, position: Int): String = JSONObject()
            .put("trackId", index).put("title", titles[index]).put("playing", playing)
            .put("volume", volume).put("positionMs", position).put("durationMs", 16000).toString()

        @JvmStatic fun command(context: Context, action: String): String {
            val active = instance
            if (active == null && (action == "state" || action == "pause" || action.startsWith("volume:"))) {
                val prefs = context.getSharedPreferences("music", MODE_PRIVATE)
                if (action.startsWith("volume:")) prefs.edit().putInt("volume", action.substringAfter(':').toInt().coerceIn(0, 100)).apply()
                return state(prefs.getInt("track", 0).coerceIn(0, 1), false, prefs.getInt("volume", 70), 0)
            }
            val id = UUID.randomUUID().toString()
            val future = CompletableFuture<String>()
            pending[id] = future
            try {
                val intent = Intent(context, MusicService::class.java).setAction(action).putExtra("request", id)
                if (active == null) context.startForegroundService(intent) else context.startService(intent)
                return future.get(8, TimeUnit.SECONDS)
            } finally { pending.remove(id) }
        }
    }
}

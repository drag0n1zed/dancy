import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:flutter/services.dart' show rootBundle;
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:dancy/src/rust/api/proxy.dart';
import 'package:dancy/src/rust/frb_generated.dart';

Future<void> main() async {
  await RustLib.init();
  runApp(const MaterialApp(home: GamePage()));
}

class GamePage extends StatefulWidget {
  const GamePage({super.key});

  @override
  State<GamePage> createState() => _GamePageState();
}

class _GamePageState extends State<GamePage> with SingleTickerProviderStateMixin {
  DancyProxy? _proxy;
  ui.Image? _frameImage;
  late Ticker _ticker;
  int _buttonState = 0xFF; // TODO: 1 -> release or 0 -> release?

  @override
  void initState() {
    super.initState();
    _startEmulator();
    _ticker = createTicker(_gameLoop);
    _ticker.start();
  }

  Future<void> _startEmulator() async {
    // Load file from assets
    final byteData = await rootBundle.load('assets/instr_timing.gb');
    final romBytes = byteData.buffer.asUint8List();

    _proxy = await DancyProxy.newInstance(romBytes: romBytes);
  }

  void _gameLoop(Duration elapsed) async {
    if (_proxy == null) return;

    // One frame
    final pixels = await _proxy!.tick();

    if (pixels.isEmpty) return;

    // Raw bytes -> GPU image
    ui.decodeImageFromPixels(
      pixels,
      160,
      144,
      ui.PixelFormat.rgba8888,
          (image) {
        if (mounted) {
          setState(() {
            _frameImage = image;
          });
        }
      },
    );
  }

  @override
  void dispose() {
    _ticker.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      body: Column(
        children: [
          // Screen
          Expanded(
            child: Center(
              child: AspectRatio(
                aspectRatio: 160 / 144,
                child: CustomPaint(
                  painter: ScreenPainter(_frameImage),
                ),
              ),
            ),
          ),
          // Controls
          SizedBox(height: 200, child: _buildControls()),
        ],
      ),
    );
  }

  Widget _buildControls() {
    // control placeholder
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceEvenly,
      children: [
        IconButton(
            icon: Icon(Icons.arrow_back, color: Colors.white, size: 40),
            onPressed: () {}), // Implement Input logic
        IconButton(
            icon: Icon(Icons.circle, color: Colors.red, size: 40),
            onPressed: () {}),
      ],
    );
  }
}

class ScreenPainter extends CustomPainter {
  final ui.Image? image;
  ScreenPainter(this.image);

  @override
  void paint(Canvas canvas, Size size) {
    if (image != null) {
      final src = Rect.fromLTWH(0, 0, 160, 144);
      final dst = Rect.fromLTWH(0, 0, size.width, size.height);
      // Nearest neighbour
      canvas.drawImageRect(image!, src, dst, Paint()..filterQuality = FilterQuality.none);
    }
  }

  @override
  bool shouldRepaint(ScreenPainter old) => true;
}
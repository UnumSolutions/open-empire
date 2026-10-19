"""Regression checks for the relay integration test's process lifecycle."""
import contextlib
import io
import os
import subprocess
import sys
import time
import unittest
from unittest import mock

import test_tools


class RelayProcessTests(unittest.TestCase):
    def setUp(self):
        self.children = []
        self.output = io.StringIO()
        popen = subprocess.Popen

        def track_child(*args, **kwargs):
            proc = popen(*args, **kwargs)
            self.children.append(proc)
            return proc

        self.enter_context(contextlib.redirect_stderr(self.output))
        self.enter_context(mock.patch.object(test_tools.subprocess, 'Popen', track_child))
        self.addCleanup(self.clean_children)

    def enter_context(self, context):
        # unittest.TestCase.enterContext is unavailable on Python 3.9.
        context.__enter__()
        self.addCleanup(context.__exit__, None, None, None)

    def clean_children(self):
        for proc in self.children:
            if proc.poll() is None:
                proc.kill()
                proc.wait(timeout=5)

    def relay(self, script, **kwargs):
        return test_tools.relay_process([sys.executable, '-u', '-c', script],
                                        shutdown_timeout=0.5, **kwargs)

    def assert_reaped(self):
        self.assertTrue(self.children)
        for proc in self.children:
            self.assertIsNotNone(proc.returncode)
            self.assertTrue(proc.stdout.closed)

    def test_normal_startup_and_cleanup(self):
        with self.relay("import time; print('listening on 127.0.0.1:1234 '); time.sleep(60)") as (proc, line):
            self.assertIn('127.0.0.1:1234', line)
            self.assertIsNone(proc.poll())
        self.assert_reaped()

    def test_silent_child_times_out_and_is_reaped(self):
        start = time.monotonic()
        with self.assertRaisesRegex(TimeoutError, 'startup timed out'):
            with self.relay('import time; time.sleep(60)', startup_timeout=0.2):
                self.fail('silent child unexpectedly started')
        self.assertLess(time.monotonic() - start, 5)
        self.assert_reaped()
        self.assertIn('stderr:', self.output.getvalue())

    def test_partial_line_does_not_bypass_deadline(self):
        with self.assertRaisesRegex(TimeoutError, 'startup timed out'):
            with self.relay("import sys, time; sys.stdout.write('listening on '); "
                            "sys.stdout.flush(); time.sleep(60)", startup_timeout=0.2):
                self.fail('partial line unexpectedly started')
        self.assert_reaped()

    def test_early_exit_reports_stderr(self):
        with self.assertRaisesRegex(RuntimeError, 'did not report a listening address'):
            with self.relay("import sys; print('bind failed', file=sys.stderr); sys.exit(7)"):
                self.fail('failed child unexpectedly started')
        self.assert_reaped()
        self.assertIn('bind failed', self.output.getvalue())

    def test_stderr_cannot_fill_a_pipe_and_block_startup(self):
        with self.relay("import sys, time; sys.stderr.write('x' * 262144); sys.stderr.flush(); "
                        "print('listening on 127.0.0.1:1234 '); time.sleep(60)"):
            pass
        self.assert_reaped()

    def test_integration_failure_survives_cleanup(self):
        failure = AssertionError('original integration failure')
        with self.assertRaises(AssertionError) as caught:
            with self.relay("import time; print('listening on 127.0.0.1:1234 '); time.sleep(60)"):
                raise failure
        self.assertIs(caught.exception, failure)
        self.assert_reaped()
        self.assertIn('Relay exit code after cleanup:', self.output.getvalue())

    def test_cleanup_error_does_not_replace_integration_failure(self):
        stop_process = test_tools.stop_process

        def fail_after_reaping(proc, timeout):
            stop_process(proc, timeout)
            raise RuntimeError('cleanup error')

        failure = AssertionError('original integration failure')
        with mock.patch.object(test_tools, 'stop_process', fail_after_reaping):
            with self.assertRaises(AssertionError) as caught:
                with self.relay("print('listening on 127.0.0.1:1234 ')"):
                    raise failure
        self.assertIs(caught.exception, failure)
        self.assert_reaped()
        self.assertIn('Relay cleanup failed: cleanup error', self.output.getvalue())

    @unittest.skipIf(os.name == 'nt', 'Windows termination cannot be ignored by a signal handler')
    def test_ignored_termination_is_killed_and_reaped(self):
        with self.relay("import signal, time; signal.signal(signal.SIGTERM, signal.SIG_IGN); "
                        "print('listening on 127.0.0.1:1234 '); time.sleep(60)"):
            pass
        self.assert_reaped()
        self.assertEqual(self.children[0].returncode, -9)


if __name__ == '__main__':
    unittest.main()

# -*- coding: utf-8 -*-
import sys, ctypes
from ctypes import c_char_p, c_uint32, Structure, POINTER
import locale
language, output_encoding = locale.getdefaultlocale()
class MtailS(Structure):
    pass

mt = ctypes.cdll.LoadLibrary('/Users/joel.self/Projects/joel/multitail-lib/target/debug/libmtaillib.dylib')
mt.multi_tail_new.restype = POINTER(MtailS)
mt.multi_tail_new.argtypes= [POINTER(ctypes.c_char_p), ctypes.c_size_t]

class FFITuple(ctypes.Structure):
		_fields_ = [("line", ctypes.c_char_p),
								("thread", ctypes.c_size_t)]
class FFITupleArray(ctypes.Structure):
    _fields_ = [("lines", POINTER(FFITuple)),
                ("len", ctypes.c_size_t)]

    @classmethod
    def from_param(cls, seq):
        return cls(seq)

def void_array_to_tuple_list(array, _func, _args):
    tuple_array = ctypes.cast(array.lines, POINTER(FFITuple))
    return [tuple_array[i] for i in range(0, array.len)]

mt.wait_for_lines.restype = FFITupleArray
mt.wait_for_lines.argtypes = (POINTER(MtailS), )
mt.wait_for_lines.errcheck = void_array_to_tuple_list

class Mtail:
	def __init__(self, files, length):
		self.obj = mt.multi_tail_new(files, length)

	def __enter__(self):
		return self

	def __exit__(self, exc_type, exc_value, traceback):
		return

	def wait_for_lines(self):
		return mt.wait_for_lines(self.obj)


args = ['/Users/joel.self/Projects/joel/test3.log', '/Users/joel.self/Projects/joel/test4.log']

c_array = (ctypes.c_char_p * len(args))(*args)
with Mtail(c_array, len(args)) as tails:
	while 1 == 1:
		lines = tails.wait_for_lines()
		for i in range(0, len(lines)):
			print "File %d => %s" % (lines[i].thread, ctypes.cast(lines[i].line, c_char_p).value),


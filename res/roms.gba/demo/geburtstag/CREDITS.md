# Credits
If I should have forgotten anything or anyone, I apologise in advance. Please let me know if possible so I can rectify it. Thanks. 

## Libraries 

### *libtonc*
Copyright 2005-2009 J Vijn

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

cf. <https://www.coranac.com/tonc/text/toc.htm> (last retrieved 2021-07-09)

### *Apex Audio System (AAS)* by Apex Designs
Copyright (c) 2003-2021 James Daniels

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

cf. <https://github.com/stuij/apex-audio-system> (last retrieved 2021-07-09)

## Math and Algorithms
### Triangle rasterisation code by Mats Byggmastar (a.k.a. MRI/Doomsday): *Fast affine texture mapping/fatmap.txt*
(In *source/render/rasteriser.h*)

It is to be noted that my modifications might have introduced erros etc. which are not present in the original version. Please refer to the original. 

cf. <http://ftp.lanet.lv/ftp/mirror/x2ftp/msdos/programming/theory/fatmap.txt> (last retrieved 2021-05-14) 

### Fast signed and unsigned division *(gba-modern)*
(In *asm/sdiv32.s* and *asm/udiv32.s*)

(Where udiv32.s is apparently taken from <https://www.chiark.greenend.org.uk/~theom/riscos/docs/ultimate/a252div.txt> (last retrieved 2021-07-09) according to the author.)

Copyright 2019-2021 João Baptista de Paula e Silva

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

cf. <https://github.com/JoaoBaptMG/gba-modern/blob/master/source/math/sdiv32.s> (last retrieved 2021-07-09)

cf. <https://github.com/JoaoBaptMG/gba-modern/blob/master/source/math/udiv32.s> (last retrieved 2021-07-09)


### *Ordering Table* (concept)
My implementation is broken, please refer to the original.

cf. <http://psx.arthus.net/sdk/Psy-Q/DOCS/TECHNOTE/ordtbl.pdf> (last retrieved 2021-07-09)


### *lookAt* function (concept)
(In *camera.c*)

cf. https://www.scratchapixel.com/lessons/mathematics-physics-for-computer-graphics/lookat-function (last retrieved 2021-07-11)


### *Object Pool* (concept)
It's not really a good use of this pattern in this case...

cf. https://gameprogrammingpatterns.com/object-pool.html (last retrieved: 2021-05-11)


### *Cohen-Sutherland* Line clipping
(In *source/render/clipping.c*)

Licensed under the [Creative Commons Attribution-ShareAlike 3.0 Unported License](https://creativecommons.org/licenses/by-sa/3.0/).

cf. <https://en.wikipedia.org/wiki/Cohen–Sutherland_algorithm> (last retrieved 2021-05-10)


### *Sutherland-Hodgman* Polygon clipping
(In *source/render/clipping.c*)

Licensed under the [Creative Commons Attribution-ShareAlike 3.0 Unported License](https://creativecommons.org/licenses/by-sa/3.0/).

cf. <https://en.wikipedia.org/wiki/Sutherland–Hodgman_algorithm> (retrieved 2021-07-09)


### Line-line intersections
(In *source/render/clipping.c*)

(Modified to avoid division by zero).

cf. <https://www.cs.helsinki.fi/group/goa/viewing/leikkaus/intro2.html>


### *Smoothstep* lerp (*Interpolation tricks* by Jari Komppa)
Implemented (and modified etc.) in *source/math.c*

cf. <https://sol.gfxile.net/interpolation/> (last retrieved 2021-07-09)


## Debugging/Misc
### *mgba_printf* by Nick Sells/adverseengineer
(In *source/logutils.c*)

cf. <https://github.com/adverseengineer/libtonc/blob/master/include/tonc_mgba.h> (last retrieved 2021-07-09)

### *REG_WAITCNT* settings 
```REG_WAITCNT = 0x4317```yields big performance improvements over the defaults in my case. (That register controls the pre-fetch buffer and the waitstates of the external cartridge bus etc.)

Thanks to the [gbadev](https://discord.io/gbadev) users who made me aware of this.

cf. <https://problemkaputt.de/gbatek.htm#gbatechnicaldata> (last retrieved 2021-05-28).


## Music
### Percussion samples are from *junglebreaks.co.uk*
The samples belong to their respective owners if applicable.  

cf. <https://archive.org/details/junglebreaks.co.uk/junglebreaks.co.uk> (last retrieved 2021-07-09)

### Instrument samples are from the *ST-01* sample pack (Karsten Obarski)
The samples belong to their respective owners if applicable. 

cf. <https://archive.org/details/AmigaSoundtrackerSamplePacksst-xx> (last retrieved 2021-07-09)

### Credits Theme
(In *assets/music/BuxWV250.mod*)

The "credits theme" is based on melodies from the *Aria in G major "La capricciosa" (BuxWV250)* by Dieterich Buxtehude (1637/39 – 1707). 

(But now I actually use it as the main theme... If you're bored, look at aaa.mod (not completly tracked) and the sheet music, it was supposed to be the main theme. We don't even have a real credits screen as of now.)


## 3d Models
### A molecule

(In *assets/models/cpa.obj*)

Credits for this model go to "Jmol development team". (Molecule generated/exported from jsmol: <http://jmol.sourceforge.net>.)

(I used Blender's decimate modifier to reduce its polygon count.)  
### *Suzanne*
(In *assets/models/suzanne.obj*)

A default model in Blender 2.8. which belongs to its respective author. Only used for debugging/profiling. 

(I used Blender's decimate modifier to reduce its polygon count.)  

cf. <https://www.blender.org> (last retrieved 2021-07-09)



## Toolchain/Tools
### *devkitARM* by devkitPro
Indispensable for building/cross-compiling. The *libtonc* which comes with devkitARM is required for using this project (as I don't bundle it).

The top level *Makefile* of this project is based on their template: <https://github.com/devkitPro/gba-examples/blob/master/template/Makefile> (last retrieved 2021-07-09)

cf. <https://devkitpro.org> (last retrieved 2021-07-09)

### *mGBA*
Indsipensable for debugging and emulation. I used version 0.9.1 on macos "Big Sur" for this project (mostly with interframe blending and bilinear filtering enabled).

cf. <https://mgba.io> (last retrieved 2021-07-09)

Add-Type -AssemblyName System.Drawing
$img=[System.Drawing.Bitmap]::FromFile('d:\aitest\football\screenshots\v8_bottom.png')
$w=$img.Width; $h=$img.Height
"size: $w x $h"
$bestY=-1; $bestCount=0
for($y=650;$y -lt 860;$y+=2){
  $g=0; $r=0
  for($x=0;$x -lt $w;$x++){
    $p=$img.GetPixel($x,$y)
    if($p.G -gt 110 -and $p.G -gt ($p.R+40) -and $p.G -gt ($p.B+40)){$g++}
    elseif($p.R -gt 150 -and $p.R -gt ($p.G+70) -and $p.R -gt ($p.B+70)){$r++}
  }
  if(($g+$r) -gt $bestCount -and $g -gt 20 -and $r -gt 20){$bestCount=$g+$r; $bestY=$y}
}
"buttonRowY=$bestY count=$bestCount"
if($bestY -ge 0){
  $gmin=$w; $gmax=-1; $rmin=$w; $rmax=-1
  for($x=0;$x -lt $w;$x++){
    $p=$img.GetPixel($x,$bestY)
    if($p.G -gt 110 -and $p.G -gt ($p.R+40) -and $p.G -gt ($p.B+40)){ if($x -lt $gmin){$gmin=$x}; if($x -gt $gmax){$gmax=$x} }
    elseif($p.R -gt 150 -and $p.R -gt ($p.G+70) -and $p.R -gt ($p.B+70)){ if($x -lt $rmin){$rmin=$x}; if($x -gt $rmax){$rmax=$x} }
  }
  $gw=$gmax-$gmin+1; $rw=$rmax-$rmin+1
  "greenX: $gmin..$gmax w=$gw"
  "redX: $rmin..$rmax w=$rw"
  $gcx=[int](($gmin+$gmax)/2); $rcx=[int](($rmin+$rmax)/2)
  $gy1=-1; $gy2=-1; $ry1=-1; $ry2=-1
  for($y=600;$y -lt 870;$y++){
    $pg=$img.GetPixel($gcx,$y)
    if($pg.G -gt 110 -and $pg.G -gt ($pg.R+40) -and $pg.G -gt ($pg.B+40)){ if($gy1 -lt 0){$gy1=$y}; $gy2=$y }
    $pr=$img.GetPixel($rcx,$y)
    if($pr.R -gt 150 -and $pr.R -gt ($pr.G+70) -and $pr.R -gt ($pr.B+70)){ if($ry1 -lt 0){$ry1=$y}; $ry2=$y }
  }
  "greenY: $gy1..$gy2 h=$($gy2-$gy1+1)"
  "redY: $ry1..$ry2 h=$($ry2-$ry1+1)"
  $center=($gmin+$rmax)/2.0
  "groupX: $gmin..$rmax center=$center imgCenter=$($w/2.0) offset=$($center-$w/2.0)"
  "leftMargin=$gmin rightMargin=$($w-$rmax-1)"
}
$img.Dispose()

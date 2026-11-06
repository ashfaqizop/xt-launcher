// Material Symbols (rounded, weight 400) as raw SVG strings. Only the icons listed
// here end up in the bundle, and nothing is fetched from the network at runtime.
import add from '@material-symbols/svg-400/rounded/add.svg?raw';
import arrowBack from '@material-symbols/svg-400/rounded/arrow_back.svg?raw';
import article from '@material-symbols/svg-400/rounded/article.svg?raw';
import check from '@material-symbols/svg-400/rounded/check.svg?raw';
import close from '@material-symbols/svg-400/rounded/close.svg?raw';
import cloudDownload from '@material-symbols/svg-400/rounded/cloud_download.svg?raw';
import deleteIcon from '@material-symbols/svg-400/rounded/delete.svg?raw';
import filterNone from '@material-symbols/svg-400/rounded/filter_none.svg?raw';
import folderOpen from '@material-symbols/svg-400/rounded/folder_open.svg?raw';
import info from '@material-symbols/svg-400/rounded/info.svg?raw';
import keyboardArrowDown from '@material-symbols/svg-400/rounded/keyboard_arrow_down.svg?raw';
import layers from '@material-symbols/svg-400/rounded/layers.svg?raw';
import manageAccounts from '@material-symbols/svg-400/rounded/manage_accounts.svg?raw';
import moreVert from '@material-symbols/svg-400/rounded/more_vert.svg?raw';
import openInNew from '@material-symbols/svg-400/rounded/open_in_new.svg?raw';
import person from '@material-symbols/svg-400/rounded/person.svg?raw';
import personAdd from '@material-symbols/svg-400/rounded/person_add.svg?raw';
import playArrow from '@material-symbols/svg-400/rounded/play_arrow.svg?raw';
import refresh from '@material-symbols/svg-400/rounded/refresh.svg?raw';
import remove from '@material-symbols/svg-400/rounded/remove.svg?raw';
import rocketLaunch from '@material-symbols/svg-400/rounded/rocket_launch.svg?raw';
import settings from '@material-symbols/svg-400/rounded/settings.svg?raw';
import cropSquare from '@material-symbols/svg-400/rounded/crop_square.svg?raw';
import wallpaper from '@material-symbols/svg-400/rounded/wallpaper.svg?raw';

export const icons = {
  add,
  back: arrowBack,
  logs: article,
  check,
  close,
  download: cloudDownload,
  trash: deleteIcon,
  restore: filterNone,
  folder: folderOpen,
  info,
  down: keyboardArrowDown,
  layers,
  accounts: manageAccounts,
  more: moreVert,
  open: openInNew,
  person,
  personAdd,
  play: playArrow,
  refresh,
  minimize: remove,
  rocket: rocketLaunch,
  settings,
  maximize: cropSquare,
  wallpaper,
} as const;

export type IconName = keyof typeof icons;

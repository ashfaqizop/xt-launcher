// Material Symbols (rounded, weight 400) as raw SVG strings. Only the icons listed
// here end up in the bundle, and nothing is fetched from the network at runtime.
import add from '@material-symbols/svg-400/rounded/add.svg?raw';
import arrowBack from '@material-symbols/svg-400/rounded/arrow_back.svg?raw';
import article from '@material-symbols/svg-400/rounded/article.svg?raw';
import check from '@material-symbols/svg-400/rounded/check.svg?raw';
import close from '@material-symbols/svg-400/rounded/close.svg?raw';
import cloudDownload from '@material-symbols/svg-400/rounded/cloud_download.svg?raw';
import contentCopy from '@material-symbols/svg-400/rounded/content_copy.svg?raw';
import edit from '@material-symbols/svg-400/rounded/edit.svg?raw';
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

import i_image from '@material-symbols/svg-400/rounded/image.svg?raw';
import i_extension from '@material-symbols/svg-400/rounded/extension.svg?raw';
import i_build from '@material-symbols/svg-400/rounded/build.svg?raw';
import i_storage from '@material-symbols/svg-400/rounded/storage.svg?raw';
import i_drag from '@material-symbols/svg-400/rounded/drag_indicator.svg?raw';
import i_search from '@material-symbols/svg-400/rounded/search.svg?raw';
import i_chevronLeft from '@material-symbols/svg-400/rounded/chevron_left.svg?raw';
import i_chevronRight from '@material-symbols/svg-400/rounded/chevron_right.svg?raw';
import i_palette from '@material-symbols/svg-400/rounded/palette.svg?raw';
import i_upload from '@material-symbols/svg-400/rounded/upload.svg?raw';
import i_schedule from '@material-symbols/svg-400/rounded/schedule.svg?raw';
import i_memory from '@material-symbols/svg-400/rounded/memory.svg?raw';
import i_tune from '@material-symbols/svg-400/rounded/tune.svg?raw';
import i_gallery from '@material-symbols/svg-400/rounded/photo_library.svg?raw';
import i_clean from '@material-symbols/svg-400/rounded/cleaning_services.svg?raw';
import i_visibility from '@material-symbols/svg-400/rounded/visibility.svg?raw';
import i_verified from '@material-symbols/svg-400/rounded/verified.svg?raw';
import i_warning from '@material-symbols/svg-400/rounded/warning.svg?raw';
import i_history from '@material-symbols/svg-400/rounded/history.svg?raw';
import i_sort from '@material-symbols/svg-400/rounded/sort.svg?raw';
import i_hardDrive from '@material-symbols/svg-400/rounded/hard_drive.svg?raw';
import i_addPhoto from '@material-symbols/svg-400/rounded/add_photo_alternate.svg?raw';
import i_checkCircle from '@material-symbols/svg-400/rounded/check_circle.svg?raw';

export const icons = {
  add,
  back: arrowBack,
  logs: article,
  check,
  close,
  download: cloudDownload,
  trash: deleteIcon,
  copy: contentCopy,
  edit,
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
  image: i_image,
  extension: i_extension,
  build: i_build,
  storage: i_storage,
  drag: i_drag,
  search: i_search,
  chevronLeft: i_chevronLeft,
  chevronRight: i_chevronRight,
  palette: i_palette,
  upload: i_upload,
  schedule: i_schedule,
  memory: i_memory,
  tune: i_tune,
  gallery: i_gallery,
  clean: i_clean,
  visibility: i_visibility,
  verified: i_verified,
  warning: i_warning,
  history: i_history,
  sort: i_sort,
  hardDrive: i_hardDrive,
  addPhoto: i_addPhoto,
  checkCircle: i_checkCircle,
} as const;

export type IconName = keyof typeof icons;
